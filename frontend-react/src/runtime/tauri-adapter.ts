import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  type AgentFileConfig,
  CHAT_STREAM_EVENT,
  type ChannelStatusResponse,
  type ChatRequest,
  type ChatStreamEnvelope,
  type ConfigMutationResponse,
  type ConfigStateResponse,
  type HealthResponse,
  type RuntimeError,
  type RuntimeErrorCode,
  type SessionState,
} from "../contracts/protocol";
import type { RuntimeAdapter } from "./adapter";

function randomStreamId(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `stream-${Date.now()}`;
}

function toRuntimeError(error: unknown): RuntimeError {
  if (error && typeof error === "object") {
    const maybeCode = (error as { code?: string }).code;
    const maybeMessage = (error as { message?: string }).message;
    if (maybeCode && maybeMessage) {
      return { code: "TAURI_INVOKE_FAILED", message: `${maybeCode}: ${maybeMessage}` };
    }
  }
  return { code: "TAURI_INVOKE_FAILED", message: String(error) };
}

function toHttpErrorCode(status: number): RuntimeErrorCode {
  if (status === 400) return "HTTP_BAD_REQUEST";
  if (status === 401 || status === 403) return "HTTP_UNAUTHORIZED";
  if (status === 404) return "HTTP_NOT_FOUND";
  if (status >= 500) return "HTTP_SERVER_ERROR";
  return "UNKNOWN";
}

function toNetworkError(error: unknown): RuntimeError {
  return { code: "NETWORK_UNAVAILABLE", message: String(error) };
}

async function requestJson<T>(url: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (error) {
    throw toNetworkError(error);
  }

  if (!response.ok) {
    throw {
      code: toHttpErrorCode(response.status),
      message: `${response.status} ${response.statusText}`,
    } as RuntimeError;
  }
  return (await response.json()) as T;
}

export function createTauriAdapter(): RuntimeAdapter {
  return {
    source: "tauri",
    async health(baseUrl: string): Promise<HealthResponse> {
      return invoke<HealthResponse>("health", { baseUrl });
    },
    async channelStatus(baseUrl: string): Promise<ChannelStatusResponse> {
      return requestJson<ChannelStatusResponse>(`${baseUrl}/api/channels/status`);
    },
    async getConfig(baseUrl: string): Promise<ConfigStateResponse> {
      return requestJson<ConfigStateResponse>(`${baseUrl}/api/config`);
    },
    async applyConfig(baseUrl: string, config: AgentFileConfig): Promise<ConfigMutationResponse> {
      return requestJson<ConfigMutationResponse>(`${baseUrl}/api/config/apply`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ config }),
      });
    },
    async resetConfig(baseUrl: string): Promise<ConfigMutationResponse> {
      return requestJson<ConfigMutationResponse>(`${baseUrl}/api/config/reset`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: "{}",
      });
    },
    async restartConfig(baseUrl: string, config?: AgentFileConfig): Promise<ConfigMutationResponse> {
      return requestJson<ConfigMutationResponse>(`${baseUrl}/api/config/restart`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(config ? { config } : {}),
      });
    },
    async listSessions(baseUrl: string): Promise<SessionState[]> {
      return invoke<SessionState[]>("list_sessions", { baseUrl });
    },
    async createSession(baseUrl: string): Promise<SessionState> {
      return invoke<SessionState>("create_session", { baseUrl });
    },
    async getSession(baseUrl: string, sessionId: string): Promise<SessionState> {
      return invoke<SessionState>("get_session", { baseUrl, sessionId });
    },
    async deleteSession(baseUrl: string, sessionId: string): Promise<void> {
      await invoke("delete_session", { baseUrl, sessionId });
    },
    async chatStream(baseUrl: string, request: ChatRequest, onEvent, onError): Promise<void> {
      const streamId = randomStreamId();

      let completeStream: () => void = () => undefined;
      let completed = false;
      const streamClosed = new Promise<void>((resolve) => {
        completeStream = () => {
          if (!completed) {
            completed = true;
            resolve();
          }
        };
      });

      const unlisten = await listen<ChatStreamEnvelope>(CHAT_STREAM_EVENT, (event) => {
        const payload = event.payload;
        if (!payload || payload.stream_id !== streamId) {
          return;
        }

        onEvent(payload);
        if (payload.event === "done" || payload.event === "error") {
          completeStream();
        }
      });

      try {
        await invoke("chat_stream", { baseUrl, request, streamId });
      } catch (error) {
        onError(toRuntimeError(error));
        completeStream();
      }

      await streamClosed;
      unlisten();
    },
  };
}
