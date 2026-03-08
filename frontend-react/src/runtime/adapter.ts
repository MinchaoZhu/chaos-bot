import type {
  AgentFileConfig,
  ChannelStatusResponse,
  ChatRequest,
  ChatStreamEnvelope,
  ConfigMutationRequest,
  ConfigMutationResponse,
  ConfigStateResponse,
  HealthResponse,
  RuntimeError,
  SessionState,
  SkillDetail,
  SkillMeta,
  UpgradeApplyResponse,
  UpgradeStatusResponse,
} from "../contracts/protocol";

export interface RuntimeAdapter {
  source: "http" | "tauri";
  health(baseUrl: string): Promise<HealthResponse>;
  channelStatus(baseUrl: string): Promise<ChannelStatusResponse>;
  listSessions(baseUrl: string): Promise<SessionState[]>;
  createSession(baseUrl: string): Promise<SessionState>;
  getSession(baseUrl: string, sessionId: string): Promise<SessionState>;
  deleteSession(baseUrl: string, sessionId: string): Promise<void>;
  getConfig(baseUrl: string): Promise<ConfigStateResponse>;
  applyConfig(baseUrl: string, req: ConfigMutationRequest): Promise<ConfigMutationResponse>;
  resetConfig(baseUrl: string): Promise<ConfigMutationResponse>;
  restartConfig(baseUrl: string, config?: AgentFileConfig): Promise<ConfigMutationResponse>;
  getUpgradeStatus(baseUrl: string): Promise<UpgradeStatusResponse>;
  applyUpgrade(baseUrl: string): Promise<UpgradeApplyResponse>;
  listSkills(baseUrl: string): Promise<SkillMeta[]>;
  getSkill(baseUrl: string, skillId: string): Promise<SkillDetail>;
  chatStream(
    baseUrl: string,
    request: ChatRequest,
    onEvent: (event: ChatStreamEnvelope) => void,
    onError: (error: RuntimeError) => void,
  ): Promise<void>;
}
