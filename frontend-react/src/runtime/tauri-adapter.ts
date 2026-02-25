import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  CHAT_STREAM_EVENT,
  type ChatRequest,
  type ChatStreamEnvelope,
  type ConfigMutationRequest,
  type ConfigMutationResponse,
  type ConfigStateResponse,
  type HealthResponse,
  type RuntimeError,
  type SessionState,
  type SkillDetail,
  type SkillMeta,
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

export function createTauriAdapter(): RuntimeAdapter {
  return {
    source: "tauri",
    async health(baseUrl: string): Promise<HealthResponse> {
      return invoke<HealthResponse>("health", { baseUrl });
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
    async getConfig(baseUrl: string): Promise<ConfigStateResponse> {
      return invoke<ConfigStateResponse>("get_config", { baseUrl });
    },
    async applyConfig(baseUrl: string, payload: ConfigMutationRequest): Promise<ConfigMutationResponse> {
      return invoke<ConfigMutationResponse>("apply_config", { baseUrl, request: payload });
    },
    async resetConfig(baseUrl: string): Promise<ConfigMutationResponse> {
      return invoke<ConfigMutationResponse>("reset_config", { baseUrl });
    },
    async restartConfig(baseUrl: string, payload?: ConfigMutationRequest): Promise<ConfigMutationResponse> {
      return invoke<ConfigMutationResponse>("restart_config", { baseUrl, request: payload ?? {} });
    },
    async listSkills(baseUrl: string): Promise<SkillMeta[]> {
      return invoke<SkillMeta[]>("list_skills", { baseUrl });
    },
    async getSkill(baseUrl: string, skillId: string): Promise<SkillDetail> {
      return invoke<SkillDetail>("get_skill", { baseUrl, skillId });
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
