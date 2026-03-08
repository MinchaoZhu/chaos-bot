import { useEffect, useState } from "react";
import type {
  AgentChannelsConfig,
  AgentFileConfig,
  AgentLoggingConfig,
  AgentLlmConfig,
  AgentSearchConfig,
  AgentSecretsConfig,
  AgentServerConfig,
  ChannelStatusResponse,
  ConfigStateResponse,
  RuntimeError,
} from "../contracts/protocol";
import type { RuntimeAdapter } from "../runtime/adapter";

type ConfigPanelProps = {
  runtime: RuntimeAdapter;
  baseUrl: string;
  health: string;
  transport: string;
  channelStatus?: ChannelStatusResponse;
  compact?: boolean;
  onBaseUrlChange: (value: string) => void;
  onLog: (summary: string) => void;
  onRuntimeError: (error: RuntimeError | undefined) => void;
};

type ConfigTab = "llm" | "search" | "connectors" | "system" | "raw";

const CONFIG_TABS: Array<{ id: ConfigTab; label: string }> = [
  { id: "llm", label: "LLM" },
  { id: "search", label: "Search" },
  { id: "connectors", label: "IM Connectors" },
  { id: "system", label: "System" },
  { id: "raw", label: "Raw" },
];

const EMPTY_CONFIG: AgentFileConfig = {
  workspace: "",
  logging: {},
  server: {},
  llm: {},
  search: {},
  channels: { telegram: {} },
  secrets: {},
};

function asText(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }
  return JSON.stringify(value) ?? String(value);
}

function asRuntimeError(value: unknown): RuntimeError {
  if (value && typeof value === "object") {
    const code = (value as { code?: string }).code;
    const message = (value as { message?: string }).message;
    if (typeof code === "string" && typeof message === "string") {
      return { code: code as RuntimeError["code"], message };
    }
  }
  return { code: "UNKNOWN", message: asText(value) };
}

function parseRawConfig(raw: string): AgentFileConfig | undefined {
  try {
    return JSON.parse(raw) as AgentFileConfig;
  } catch {
    return undefined;
  }
}

function formatConfig(config: AgentFileConfig): string {
  return `${JSON.stringify(config, null, 2)}\n`;
}

function normalizedConfig(config?: AgentFileConfig): AgentFileConfig {
  return {
    workspace: config?.workspace ?? "",
    logging: { ...(config?.logging ?? {}) } as AgentLoggingConfig,
    server: { ...(config?.server ?? {}) } as AgentServerConfig,
    llm: { ...(config?.llm ?? {}) } as AgentLlmConfig,
    search: { ...(config?.search ?? {}) } as AgentSearchConfig,
    channels: {
      telegram: { ...(config?.channels?.telegram ?? {}) },
    } as AgentChannelsConfig,
    secrets: { ...(config?.secrets ?? {}) } as AgentSecretsConfig,
  };
}

function asOptionalString(value: string): string | undefined {
  const next = value.trim();
  return next ? next : undefined;
}

function asOptionalInt(value: string): number | undefined {
  const next = value.trim();
  if (!next) {
    return undefined;
  }
  const parsed = Number.parseInt(next, 10);
  return Number.isNaN(parsed) ? undefined : parsed;
}

function asOptionalFloat(value: string): number | undefined {
  const next = value.trim();
  if (!next) {
    return undefined;
  }
  const parsed = Number.parseFloat(next);
  return Number.isNaN(parsed) ? undefined : parsed;
}

export function ConfigPanel({
  runtime,
  baseUrl,
  health,
  transport,
  channelStatus,
  compact,
  onBaseUrlChange,
  onLog,
  onRuntimeError,
}: ConfigPanelProps) {
  const [state, setState] = useState<ConfigStateResponse>();
  const [raw, setRaw] = useState("");
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState(false);
  const [action, setAction] = useState<"apply" | "reset" | "restart" | undefined>();
  const [activeTab, setActiveTab] = useState<ConfigTab>("llm");

  const busy = loading || Boolean(action);
  const parsedConfig = parseRawConfig(raw);
  const config = normalizedConfig(parsedConfig ?? state?.running ?? EMPTY_CONFIG);

  function updateConfig(mutator: (current: AgentFileConfig) => AgentFileConfig) {
    setRaw((current) => {
      const base = normalizedConfig(parseRawConfig(current) ?? state?.running ?? EMPTY_CONFIG);
      return formatConfig(mutator(base));
    });
  }

  async function loadConfig() {
    setLoading(true);
    try {
      const next = await runtime.getConfig(baseUrl);
      setState(next);
      setRaw(next.raw);
      setStatus("config loaded");
      onRuntimeError(undefined);
      onLog(`[config.get] ${next.config_path}`);
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`load failed: ${runtimeError.message}`);
    } finally {
      setLoading(false);
    }
  }

  async function applyConfig() {
    setAction("apply");
    try {
      const response = await runtime.applyConfig(baseUrl, { raw });
      setState(response.state);
      setRaw(response.state.raw);
      setStatus(`apply ok (restart_scheduled=${response.restart_scheduled})`);
      onRuntimeError(undefined);
      onLog(`[config.apply] restart_scheduled=${response.restart_scheduled}`);
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`apply failed: ${runtimeError.message}`);
    } finally {
      setAction(undefined);
    }
  }

  async function resetConfig() {
    setAction("reset");
    try {
      const response = await runtime.resetConfig(baseUrl);
      setState(response.state);
      setRaw(response.state.raw);
      setStatus(`reset ok (restart_scheduled=${response.restart_scheduled})`);
      onRuntimeError(undefined);
      onLog("[config.reset]");
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`reset failed: ${runtimeError.message}`);
    } finally {
      setAction(undefined);
    }
  }

  async function restartRuntime() {
    setAction("restart");
    try {
      const response = await runtime.restartConfig(baseUrl);
      setState(response.state);
      setRaw(response.state.raw);
      setStatus(`restart ok (restart_scheduled=${response.restart_scheduled})`);
      onRuntimeError(undefined);
      onLog(`[config.restart] restart_scheduled=${response.restart_scheduled}`);
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`restart failed: ${runtimeError.message}`);
    } finally {
      setAction(undefined);
    }
  }

  useEffect(() => {
    void loadConfig();
    // Re-load when runtime target changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [baseUrl, runtime]);

  return (
    <section className={`panel config-panel ${compact ? "compact" : ""}`}>
      <div className="panel-head">
        <h2>Config</h2>
        <button type="button" className="ghost-btn" onClick={() => void loadConfig()} disabled={busy}>
          {loading ? "Loading..." : "Reload Config"}
        </button>
      </div>

      <div className="config-meta">
        <p>
          config <strong>{state?.config_format ?? "-"}</strong>
        </p>
        <p>
          provider <strong>{config.llm.provider ?? "-"}</strong>
        </p>
        <p>
          model <strong>{config.llm.model ?? "-"}</strong>
        </p>
      </div>

      <nav className="config-tabs" aria-label="Config sections">
        {CONFIG_TABS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            className={activeTab === tab.id ? "active" : ""}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </nav>

      {activeTab === "llm" ? (
        <section className="config-section">
          <div className="field-grid two">
            <label className="base-url">
              <span>Provider</span>
              <input
                value={config.llm.provider ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    llm: { ...current.llm, provider: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="openai"
              />
            </label>
            <label className="base-url">
              <span>Model</span>
              <input
                value={config.llm.model ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    llm: { ...current.llm, model: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="gpt-4o-mini"
              />
            </label>
            <label className="base-url">
              <span>Temperature</span>
              <input
                value={config.llm.temperature?.toString() ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    llm: { ...current.llm, temperature: asOptionalFloat(event.target.value) },
                  }))
                }
                placeholder="0.2"
              />
            </label>
            <label className="base-url">
              <span>Max Tokens</span>
              <input
                value={config.llm.max_tokens?.toString() ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    llm: { ...current.llm, max_tokens: asOptionalInt(event.target.value) },
                  }))
                }
                placeholder="1024"
              />
            </label>
            <label className="base-url">
              <span>Max Iterations</span>
              <input
                value={config.llm.max_iterations?.toString() ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    llm: { ...current.llm, max_iterations: asOptionalInt(event.target.value) },
                  }))
                }
                placeholder="6"
              />
            </label>
            <label className="base-url">
              <span>Token Budget</span>
              <input
                value={config.llm.token_budget?.toString() ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    llm: { ...current.llm, token_budget: asOptionalInt(event.target.value) },
                  }))
                }
                placeholder="12000"
              />
            </label>
          </div>

          <div className="field-grid three">
            <label className="base-url">
              <span>OpenAI API Key</span>
              <input
                value={config.secrets.openai_api_key ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, openai_api_key: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="sk-..."
              />
            </label>
            <label className="base-url">
              <span>Anthropic API Key</span>
              <input
                value={config.secrets.anthropic_api_key ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, anthropic_api_key: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="sk-ant-..."
              />
            </label>
            <label className="base-url">
              <span>Gemini API Key</span>
              <input
                value={config.secrets.gemini_api_key ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, gemini_api_key: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="AIza..."
              />
            </label>
          </div>
        </section>
      ) : null}

      {activeTab === "search" ? (
        <section className="config-section">
          <div className="field-grid two">
            <label className="base-url">
              <span>Provider</span>
              <select
                data-testid="search-provider-select"
                value={config.search.provider ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    search: { ...current.search, provider: asOptionalString(event.target.value) },
                  }))
                }
              >
                <option value="">None</option>
                <option value="perplexity">Perplexity</option>
                <option value="tavily">Tavily</option>
                <option value="brave">Brave</option>
              </select>
            </label>
          </div>

          <div className="field-grid three">
            <label className="base-url">
              <span>Perplexity API Key</span>
              <input
                value={config.secrets.perplexity_api_key ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, perplexity_api_key: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="pplx-..."
              />
            </label>
            <label className="base-url">
              <span>Tavily API Key</span>
              <input
                value={config.secrets.tavily_api_key ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, tavily_api_key: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="tvly-..."
              />
            </label>
            <label className="base-url">
              <span>Brave Search API Key</span>
              <input
                value={config.secrets.brave_search_api_key ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, brave_search_api_key: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="brave-search-key"
              />
            </label>
          </div>
        </section>
      ) : null}

      {activeTab === "connectors" ? (
        <section className="config-section connector-config">
          <h3>Telegram</h3>
          <div className="connector-toggles">
            <label>
              <input
                type="checkbox"
                checked={config.channels.telegram.enabled ?? false}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    channels: {
                      ...current.channels,
                      telegram: { ...current.channels.telegram, enabled: event.target.checked },
                    },
                  }))
                }
              />
              <span>enabled</span>
            </label>
            <label>
              <input
                type="checkbox"
                checked={config.channels.telegram.polling ?? false}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    channels: {
                      ...current.channels,
                      telegram: { ...current.channels.telegram, polling: event.target.checked },
                    },
                  }))
                }
              />
              <span>polling mode</span>
            </label>
          </div>
          <div className="field-grid two">
            <label className="base-url">
              <span>API Base URL</span>
              <input
                value={config.channels.telegram.api_base_url ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    channels: {
                      ...current.channels,
                      telegram: {
                        ...current.channels.telegram,
                        api_base_url: asOptionalString(event.target.value),
                      },
                    },
                  }))
                }
                placeholder="https://api.telegram.org"
              />
            </label>
            <label className="base-url">
              <span>Bot Token</span>
              <input
                value={config.secrets.telegram_bot_token ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    secrets: { ...current.secrets, telegram_bot_token: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="BotFather token"
              />
            </label>
            <label className="base-url">
              <span>Webhook Secret</span>
              <input
                value={config.channels.telegram.webhook_secret ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    channels: {
                      ...current.channels,
                      telegram: {
                        ...current.channels.telegram,
                        webhook_secret: asOptionalString(event.target.value),
                      },
                    },
                  }))
                }
                placeholder="x-telegram-bot-api-secret-token"
              />
            </label>
            <label className="base-url">
              <span>Webhook Base URL</span>
              <input
                value={config.channels.telegram.webhook_base_url ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    channels: {
                      ...current.channels,
                      telegram: {
                        ...current.channels.telegram,
                        webhook_base_url: asOptionalString(event.target.value),
                      },
                    },
                  }))
                }
                placeholder="https://example.com"
              />
            </label>
          </div>
        </section>
      ) : null}

      {activeTab === "system" ? (
        <section className="config-section">
          <div className="config-meta">
            <p>
              transport <strong>{transport}</strong>
            </p>
            <p>
              health <strong>{health}</strong>
            </p>
            <p>
              channels <strong>{channelStatus?.enabled_channels.join(", ") || "none"}</strong>
            </p>
          </div>

          <div className="field-grid two">
            <label className="base-url">
              <span>Backend URL</span>
              <input value={baseUrl} disabled={busy} onChange={(event) => onBaseUrlChange(event.target.value)} />
            </label>
            <label className="base-url">
              <span>Workspace</span>
              <input
                value={typeof config.workspace === "string" ? config.workspace : ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    workspace: asOptionalString(event.target.value),
                  }))
                }
                placeholder="~/.chaos-bot"
              />
            </label>
            <label className="base-url">
              <span>Server Host</span>
              <input
                value={config.server.host ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    server: { ...current.server, host: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="0.0.0.0"
              />
            </label>
            <label className="base-url">
              <span>Server Port</span>
              <input
                value={config.server.port?.toString() ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    server: { ...current.server, port: asOptionalInt(event.target.value) },
                  }))
                }
                placeholder="3000"
              />
            </label>
            <label className="base-url">
              <span>Logging Level</span>
              <input
                value={config.logging.level ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    logging: { ...current.logging, level: asOptionalString(event.target.value) },
                  }))
                }
                placeholder="info"
              />
            </label>
            <label className="base-url">
              <span>Retention Days</span>
              <input
                value={config.logging.retention_days?.toString() ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    logging: {
                      ...current.logging,
                      retention_days: asOptionalInt(event.target.value),
                    },
                  }))
                }
                placeholder="7"
              />
            </label>
            <label className="base-url field-span-two">
              <span>Logging Directory</span>
              <input
                value={config.logging.directory ?? ""}
                disabled={busy}
                onChange={(event) =>
                  updateConfig((current) => ({
                    ...current,
                    logging: {
                      ...current.logging,
                      directory: asOptionalString(event.target.value),
                    },
                  }))
                }
                placeholder="~/.chaos-bot/logs"
              />
            </label>
          </div>
        </section>
      ) : null}

      {activeTab === "raw" ? (
        <section className="config-section">
          <label className="base-url">
            <span>Raw JSON</span>
            <textarea
              className="config-editor"
              data-testid="config-raw-editor"
              value={raw}
              onChange={(event) => setRaw(event.target.value)}
              disabled={busy}
              placeholder="Runtime config JSON"
            />
          </label>
        </section>
      ) : null}

      <div className="config-actions">
        <button type="button" onClick={() => void applyConfig()} disabled={busy || !raw.trim()}>
          Apply Config
        </button>
        <button type="button" className="ghost-btn" onClick={() => void resetConfig()} disabled={busy}>
          Reset Config
        </button>
        <button type="button" className="ghost-btn" onClick={() => void restartRuntime()} disabled={busy}>
          Restart Runtime
        </button>
      </div>

      {status ? <p className="config-status">{status}</p> : null}
      {state?.disk_parse_error ? (
        <div className="runtime-error">disk_parse_error: {state.disk_parse_error}</div>
      ) : null}
      {!parsedConfig && raw.trim() ? (
        <div className="runtime-error">raw JSON is invalid; structured tabs show the last known good config.</div>
      ) : null}
    </section>
  );
}
