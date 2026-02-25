export const CHAT_STREAM_EVENT = "chaos://chat-event";

export type RuntimeCommand =
  | "runtime.health"
  | "session.list"
  | "session.create"
  | "session.get"
  | "session.delete"
  | "chat.stream"
  | "config.get"
  | "config.apply"
  | "config.reset"
  | "config.restart";

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

export interface AgentFileConfig {
  workspace: string;
  server: {
    host: string;
    port: number;
  };
  llm: {
    provider: string;
    model: string;
    temperature: number;
    max_tokens: number;
    max_iterations: number;
    token_budget: number;
  };
  logging: {
    level: string;
    retention_days: number;
    directory: string;
  };
  secrets: {
    openai_api_key?: string | null;
    anthropic_api_key?: string | null;
    gemini_api_key?: string | null;
  };
}

export interface ConfigStateResponse {
  config_path: string;
  backup1_path: string;
  backup2_path: string;
  config_format: string;
  running: AgentFileConfig;
  disk: AgentFileConfig;
  raw: string;
  disk_parse_error?: string | null;
}

export interface ConfigMutationResponse {
  ok: boolean;
  action: string;
  restart_scheduled: boolean;
  state: ConfigStateResponse;
}

export interface ConfigMutationRequest {
  raw?: string;
  config?: AgentFileConfig;
}
