export const CHAT_STREAM_EVENT = "chaos://chat-event";

export type RuntimeCommand =
  | "runtime.health"
  | "channel.status"
  | "config.get"
  | "config.apply"
  | "config.reset"
  | "config.restart"
  | "upgrade.status"
  | "upgrade.apply"
  | "upgrade.relaunch"
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

export interface ChannelHealth {
  channel: string;
  status: string;
  detail: Record<string, unknown>;
}

export interface TelegramChannelStatus {
  enabled: boolean;
  polling: boolean;
  api_base_url: string;
  webhook_secret_configured: boolean;
}

export interface ChannelStatusResponse {
  enabled_channels: string[];
  connectors: ChannelHealth[];
  telegram: TelegramChannelStatus;
}

export interface AgentServerConfig {
  host?: string;
  port?: number;
}

export interface AgentLlmConfig {
  provider?: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
  max_iterations?: number;
  token_budget?: number;
}

export interface AgentSearchConfig {
  provider?: string;
}

export interface AgentLoggingConfig {
  level?: string;
  retention_days?: number;
  directory?: string;
}

export interface AgentTelegramConfig {
  enabled?: boolean;
  webhook_secret?: string;
  webhook_base_url?: string;
  polling?: boolean;
  api_base_url?: string;
}

export interface AgentChannelsConfig {
  telegram: AgentTelegramConfig;
}

export interface AgentSecretsConfig {
  openai_api_key?: string;
  anthropic_api_key?: string;
  gemini_api_key?: string;
  perplexity_api_key?: string;
  tavily_api_key?: string;
  brave_search_api_key?: string;
  telegram_bot_token?: string;
}

export interface AgentFileConfig {
  workspace?: string;
  logging: AgentLoggingConfig;
  server: AgentServerConfig;
  llm: AgentLlmConfig;
  search: AgentSearchConfig;
  channels: AgentChannelsConfig;
  secrets: AgentSecretsConfig;
}

export interface ConfigStateResponse {
  config_path: string;
  backup1_path: string;
  backup2_path: string;
  config_format: string;
  running: AgentFileConfig;
  disk: AgentFileConfig;
  raw: string;
  disk_parse_error?: string;
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

export interface UpgradeStatusResponse {
  supported: boolean;
  current_version?: string;
  latest_version?: string;
  latest_tag_name?: string;
  upgrade_available: boolean;
  install_prefix?: string;
  current_release_root?: string;
  repository?: string;
  latest_release_url?: string;
  download_url?: string;
  reason?: string;
}

export interface UpgradeApplyResponse {
  ok: boolean;
  action: string;
  current_version?: string;
  target_version?: string;
  launcher_path?: string;
  installed_release_root?: string;
  relaunch_required: boolean;
  message: string;
  status: UpgradeStatusResponse;
}

export interface UpgradeRestartResponse {
  ok: boolean;
  action: string;
  launcher_path?: string;
  target_version?: string;
  message: string;
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

export interface SkillMeta {
  id: string;
  name: string;
  description: string;
}

export interface SkillDetail {
  meta: SkillMeta;
  body: string;
}
