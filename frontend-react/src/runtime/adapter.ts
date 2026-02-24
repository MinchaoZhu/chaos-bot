import type {
  ChatRequest,
  ChatStreamEnvelope,
  HealthResponse,
  RuntimeError,
  SessionState,
} from "../contracts/protocol";

export interface RuntimeAdapter {
  source: "http" | "tauri";
  health(baseUrl: string): Promise<HealthResponse>;
  listSessions(baseUrl: string): Promise<SessionState[]>;
  createSession(baseUrl: string): Promise<SessionState>;
  getSession(baseUrl: string, sessionId: string): Promise<SessionState>;
  deleteSession(baseUrl: string, sessionId: string): Promise<void>;
  chatStream(
    baseUrl: string,
    request: ChatRequest,
    onEvent: (event: ChatStreamEnvelope) => void,
    onError: (error: RuntimeError) => void,
  ): Promise<void>;
}
