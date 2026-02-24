export const CHAT_STREAM_EVENT = "chaos://chat-event";

export type RuntimeCommand =
  | "runtime.health"
  | "session.list"
  | "session.create"
  | "session.get"
  | "session.delete"
  | "chat.stream";

export type StreamEventType = "session" | "delta" | "tool_call" | "done" | "error";

export type RuntimeErrorCode =
  | "NETWORK_UNAVAILABLE"
  | "HTTP_BAD_REQUEST"
  | "HTTP_UNAUTHORIZED"
  | "HTTP_NOT_FOUND"
  | "HTTP_SERVER_ERROR"
  | "SSE_PROTOCOL_ERROR"
  | "TAURI_INVOKE_FAILED"
  | "UNKNOWN";

export interface RuntimeError {
  code: RuntimeErrorCode;
  message: string;
}

export interface HealthResponse {
  status: "ok";
  now: string;
}

export interface ChatRequest {
  session_id?: string;
  message: string;
}

export interface SessionMessage {
  role: string;
  content?: string;
  tool_name?: string;
  tool_call_id?: string;
}

export interface SessionState {
  id: string;
  messages: SessionMessage[];
  created_at: string;
  updated_at: string;
}

export interface ChatStreamEnvelope {
  stream_id: string;
  event: StreamEventType;
  data: unknown;
}
