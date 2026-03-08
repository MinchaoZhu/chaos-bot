import { useEffect, useState } from "react";
import type {
  AgentFileConfig,
  ConfigStateResponse,
  RuntimeError,
  UpgradeStatusResponse,
} from "../contracts/protocol";
import type { RuntimeAdapter } from "../runtime/adapter";

type ConfigPanelProps = {
  runtime: RuntimeAdapter;
  baseUrl: string;
  compact?: boolean;
  onLog: (summary: string) => void;
  onRuntimeError: (error: RuntimeError | undefined) => void;
};

type SearchProvider = "" | "perplexity" | "tavily" | "brave";

type SearchDraft = {
  provider: SearchProvider;
  perplexityApiKey: string;
  tavilyApiKey: string;
  braveSearchApiKey: string;
};

const EMPTY_SEARCH_DRAFT: SearchDraft = {
  provider: "",
  perplexityApiKey: "",
  tavilyApiKey: "",
  braveSearchApiKey: "",
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

function searchDraftFromConfig(config?: AgentFileConfig): SearchDraft {
  const provider = (config?.search?.provider ?? "").toLowerCase();
  return {
    provider:
      provider === "perplexity" || provider === "tavily" || provider === "brave"
        ? provider
        : "",
    perplexityApiKey: config?.secrets?.perplexity_api_key ?? "",
    tavilyApiKey: config?.secrets?.tavily_api_key ?? "",
    braveSearchApiKey: config?.secrets?.brave_search_api_key ?? "",
  };
}

function syncSearchIntoRaw(raw: string, draft: SearchDraft): string {
  try {
    const parsed = JSON.parse(raw) as AgentFileConfig;
    const nextConfig: AgentFileConfig = {
      ...parsed,
      search: {
        ...(parsed.search ?? {}),
        provider: draft.provider || undefined,
      },
      secrets: {
        ...(parsed.secrets ?? {}),
        perplexity_api_key: draft.perplexityApiKey.trim() || undefined,
        tavily_api_key: draft.tavilyApiKey.trim() || undefined,
        brave_search_api_key: draft.braveSearchApiKey.trim() || undefined,
      },
    };
    return `${JSON.stringify(nextConfig, null, 2)}\n`;
  } catch {
    return raw;
  }
}

export function ConfigPanel({
  runtime,
  baseUrl,
  compact,
  onLog,
  onRuntimeError,
}: ConfigPanelProps) {
  const [state, setState] = useState<ConfigStateResponse>();
  const [upgradeStatus, setUpgradeStatus] = useState<UpgradeStatusResponse>();
  const [raw, setRaw] = useState("");
  const [searchDraft, setSearchDraft] = useState<SearchDraft>(EMPTY_SEARCH_DRAFT);
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState(false);
  const [action, setAction] = useState<"apply" | "reset" | "restart" | "upgrade" | undefined>();

  const busy = loading || Boolean(action);

  async function loadConfig() {
    setLoading(true);
    try {
      const [next, nextUpgradeStatus] = await Promise.all([
        runtime.getConfig(baseUrl),
        runtime.getUpgradeStatus(baseUrl),
      ]);
      setState(next);
      setUpgradeStatus(nextUpgradeStatus);
      setRaw(next.raw);
      setSearchDraft(searchDraftFromConfig(next.running));
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
      setSearchDraft(searchDraftFromConfig(response.state.running));
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

  async function refreshUpgradeStatus() {
    try {
      const next = await runtime.getUpgradeStatus(baseUrl);
      setUpgradeStatus(next);
      setStatus(
        next.supported
          ? next.upgrade_available
            ? `upgrade available: ${next.current_version ?? "unknown"} -> ${next.latest_version ?? "unknown"}`
            : `upgrade status ok (${next.current_version ?? "unknown"})`
          : `upgrade unavailable: ${next.reason ?? "unsupported runtime"}`,
      );
      onRuntimeError(undefined);
      onLog(`[upgrade.status] available=${next.upgrade_available}`);
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`upgrade status failed: ${runtimeError.message}`);
    }
  }

  async function applyUpgrade() {
    setAction("upgrade");
    try {
      const response = await runtime.applyUpgrade(baseUrl);
      setUpgradeStatus(response.status);
      setStatus(response.message);
      onRuntimeError(undefined);
      onLog(
        `[upgrade.apply] action=${response.action} current=${response.current_version ?? "-"} target=${response.target_version ?? "-"}`,
      );
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`upgrade failed: ${runtimeError.message}`);
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
      setSearchDraft(searchDraftFromConfig(response.state.running));
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
      setSearchDraft(searchDraftFromConfig(response.state.running));
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

  useEffect(() => {
    setRaw((current) => syncSearchIntoRaw(current, searchDraft));
  }, [searchDraft]);

  return (
    <section className={`panel config-panel ${compact ? "compact" : ""}`}>
      <div className="panel-head">
        <h2>Runtime Config</h2>
        <button type="button" className="ghost-btn" onClick={() => void loadConfig()} disabled={busy}>
          {loading ? "Loading..." : "Reload Config"}
        </button>
      </div>

      <div className="config-meta">
        <p>
          config <strong>{state?.config_format ?? "-"}</strong>
        </p>
        <p>
          provider <strong>{state?.running.llm.provider ?? "-"}</strong>
        </p>
        <p>
          model <strong>{state?.running.llm.model ?? "-"}</strong>
        </p>
      </div>

      <section className="config-structured-section">
        <div className="panel-head">
          <h3>Search</h3>
        </div>

        <label className="base-url">
          <span>Provider</span>
          <select
            data-testid="search-provider-select"
            value={searchDraft.provider}
            disabled={busy}
            onChange={(event) =>
              setSearchDraft((prev) => ({
                ...prev,
                provider: event.target.value as SearchProvider,
              }))
            }
          >
            <option value="">None</option>
            <option value="perplexity">Perplexity</option>
            <option value="tavily">Tavily</option>
            <option value="brave">Brave</option>
          </select>
        </label>

        {searchDraft.provider === "perplexity" ? (
          <label className="base-url">
            <span>Perplexity API Key</span>
            <input
              data-testid="search-provider-key"
              value={searchDraft.perplexityApiKey}
              disabled={busy}
              onChange={(event) =>
                setSearchDraft((prev) => ({ ...prev, perplexityApiKey: event.target.value }))
              }
              placeholder="pplx-..."
            />
          </label>
        ) : null}

        {searchDraft.provider === "tavily" ? (
          <label className="base-url">
            <span>Tavily API Key</span>
            <input
              data-testid="search-provider-key"
              value={searchDraft.tavilyApiKey}
              disabled={busy}
              onChange={(event) =>
                setSearchDraft((prev) => ({ ...prev, tavilyApiKey: event.target.value }))
              }
              placeholder="tvly-..."
            />
          </label>
        ) : null}

        {searchDraft.provider === "brave" ? (
          <label className="base-url">
            <span>Brave Search API Key</span>
            <input
              data-testid="search-provider-key"
              value={searchDraft.braveSearchApiKey}
              disabled={busy}
              onChange={(event) =>
                setSearchDraft((prev) => ({ ...prev, braveSearchApiKey: event.target.value }))
              }
              placeholder="brave-search-key"
            />
          </label>
        ) : null}
      </section>

      <section className="config-structured-section">
        <div className="panel-head">
          <h3>Self Upgrade</h3>
          <button type="button" className="ghost-btn" onClick={() => void refreshUpgradeStatus()} disabled={busy}>
            Refresh Upgrade Status
          </button>
        </div>

        <div className="config-meta">
          <p>
            installed <strong>{upgradeStatus?.current_version ?? "-"}</strong>
          </p>
          <p>
            latest <strong>{upgradeStatus?.latest_version ?? "-"}</strong>
          </p>
          <p>
            available <strong>{upgradeStatus?.upgrade_available ? "yes" : "no"}</strong>
          </p>
        </div>

        <p className="config-status">
          {upgradeStatus?.supported
            ? upgradeStatus.reason ?? "Installed release bundles can download and install newer GitHub releases. Relaunch is required after a successful upgrade."
            : upgradeStatus?.reason ?? "Self-upgrade is only available from an installed release bundle."}
        </p>

        {upgradeStatus?.repository ? <p>repository: {upgradeStatus.repository}</p> : null}
        {upgradeStatus?.install_prefix ? <p>prefix: {upgradeStatus.install_prefix}</p> : null}

        <div className="config-actions">
          <button
            type="button"
            data-testid="upgrade-apply-button"
            onClick={() => void applyUpgrade()}
            disabled={busy || !upgradeStatus?.supported || !upgradeStatus.upgrade_available}
          >
            Install Latest Release
          </button>
        </div>
      </section>

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
    </section>
  );
}
