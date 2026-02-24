import type {
  ChatRequest,
  ChatStreamEnvelope,
  HealthResponse,
  RuntimeError,
  RuntimeErrorCode,
  SessionState,
  StreamEventType,
} from "../contracts/protocol";
import type { RuntimeAdapter } from "./adapter";

const FALLBACK_STREAM_ID = "http-stream";

function toHttpErrorCode(status: number): RuntimeErrorCode {
  if (status === 400) return "HTTP_BAD_REQUEST";
  if (status === 401 || status === 403) return "HTTP_UNAUTHORIZED";
  if (status === 404) return "HTTP_NOT_FOUND";
  if (status >= 500) return "HTTP_SERVER_ERROR";
  return "UNKNOWN";
}

function toRuntimeError(code: RuntimeErrorCode, message: string): RuntimeError {
  return { code, message };
}

function parseSseBlock(block: string): { event: StreamEventType; data: string } {
  const lines = block.split("\n");
  let event: StreamEventType = "delta";
  let data = "";

  for (const line of lines) {
    if (line.startsWith("event:")) {
      const value = line.slice(6).trim();
      if (
        value === "session" ||
        value === "delta" ||
        value === "tool_call" ||
        value === "done" ||
        value === "error"
      ) {
        event = value;
      }
    }
    if (line.startsWith("data:")) {
      data += line.slice(5).trim();
    }
  }

  return { event, data };
}

async function requestJson<T>(url: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (error) {
    throw toRuntimeError("NETWORK_UNAVAILABLE", String(error));
  }

  if (!response.ok) {
    throw toRuntimeError(toHttpErrorCode(response.status), `${response.status} ${response.statusText}`);
  }

  return (await response.json()) as T;
}

async function requestWithoutBody(url: string, init?: RequestInit): Promise<void> {
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (error) {
    throw toRuntimeError("NETWORK_UNAVAILABLE", String(error));
  }

  if (!response.ok) {
    throw toRuntimeError(toHttpErrorCode(response.status), `${response.status} ${response.statusText}`);
  }
}

export function createHttpAdapter(): RuntimeAdapter {
  return {
    source: "http",
    async health(baseUrl: string): Promise<HealthResponse> {
      return requestJson<HealthResponse>(`${baseUrl}/api/health`);
    },
    async listSessions(baseUrl: string): Promise<SessionState[]> {
      return requestJson<SessionState[]>(`${baseUrl}/api/sessions`);
    },
    async createSession(baseUrl: string): Promise<SessionState> {
      return requestJson<SessionState>(`${baseUrl}/api/sessions`, { method: "POST" });
    },
    async getSession(baseUrl: string, sessionId: string): Promise<SessionState> {
      return requestJson<SessionState>(`${baseUrl}/api/sessions/${sessionId}`);
    },
    async deleteSession(baseUrl: string, sessionId: string): Promise<void> {
      await requestWithoutBody(`${baseUrl}/api/sessions/${sessionId}`, { method: "DELETE" });
    },
    async chatStream(baseUrl: string, request: ChatRequest, onEvent, onError): Promise<void> {
      let response: Response;
      try {
        response = await fetch(`${baseUrl}/api/chat`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(request),
        });
      } catch (error) {
        onError(toRuntimeError("NETWORK_UNAVAILABLE", String(error)));
        return;
      }

      if (!response.ok || !response.body) {
        onError(
          toRuntimeError(
            toHttpErrorCode(response.status),
            `chat stream failed: ${response.status} ${response.statusText}`,
          ),
        );
        return;
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        const blocks = buffer.split("\n\n");
        buffer = blocks.pop() ?? "";

        for (const block of blocks) {
          if (!block.trim() || block.includes("keepalive")) {
            continue;
          }

          const parsed = parseSseBlock(block);
          let payload: unknown = parsed.data;
          if (["session", "tool_call", "done", "error"].includes(parsed.event)) {
            try {
              payload = JSON.parse(parsed.data);
            } catch (error) {
              onError(toRuntimeError("SSE_PROTOCOL_ERROR", `invalid JSON payload: ${String(error)}`));
              continue;
            }
          }

          const envelope: ChatStreamEnvelope = {
            stream_id: FALLBACK_STREAM_ID,
            event: parsed.event,
            data: payload,
          };

          onEvent(envelope);
        }
      }
    },
  };
}
