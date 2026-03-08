import { useEffect, useState } from "react";
import type {
  AgentFileConfig,
  ConfigStateResponse,
  RuntimeError,
  UpgradeApplyResponse,
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
  const [action, setAction] = useState<"apply" | "reset" | "restart" | "upgrade" | "relaunch" | undefined>();
  const [lastUpgradeResult, setLastUpgradeResult] = useState<UpgradeApplyResponse>();

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
      setLastUpgradeResult(response);
      setUpgradeStatus(response.status);
      setStatus(
        response.relaunch_required
          ? `Upgrade installed successfully. Restart ${response.launcher_path ?? "~/.local/bin/chaos-bot"} to use ${response.target_version ?? "the new release"}.`
          : response.message,
      );
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

  async function relaunchUpgrade() {
    setAction("relaunch");
    try {
      const response = await runtime.relaunchUpgrade(baseUrl);
      setStatus(
        `Restart requested successfully. chaos-bot is relaunching via ${response.launcher_path ?? "~/.local/bin/chaos-bot"}.`,
      );
      onRuntimeError(undefined);
      onLog(`[upgrade.relaunch] target=${response.target_version ?? "-"}`);
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`restart failed: ${runtimeError.message}`);
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

  async function applySearchConfig() {
    if (!state) {
      return;
    }

    setAction("apply");
    try {
      const nextConfig: AgentFileConfig = {
        ...state.running,
        search: {
          ...(state.running.search ?? {}),
          provider: searchDraft.provider || undefined,
        },
        secrets: {
          ...state.running.secrets,
          perplexity_api_key: searchDraft.perplexityApiKey.trim() || undefined,
          tavily_api_key: searchDraft.tavilyApiKey.trim() || undefined,
          brave_search_api_key: searchDraft.braveSearchApiKey.trim() || undefined,
        },
      };
      const response = await runtime.applyConfig(baseUrl, { config: nextConfig });
      setState(response.state);
      setRaw(response.state.raw);
      setSearchDraft(searchDraftFromConfig(response.state.running));
      setStatus(`search apply ok (restart_scheduled=${response.restart_scheduled})`);
      onRuntimeError(undefined);
      onLog(
        `[config.apply.search] provider=${response.state.running.search.provider ?? "none"} restart_scheduled=${response.restart_scheduled}`,
      );
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setStatus(`search apply failed: ${runtimeError.message}`);
    } finally {
      setAction(undefined);
    }
  }

  const upgradeHeadline = !upgradeStatus
    ? "Checking installed release..."
    : !upgradeStatus.supported
      ? "Web UI upgrade is unavailable in this runtime"
      : upgradeStatus.upgrade_available
        ? `Upgrade ready: ${upgradeStatus.current_version ?? "unknown"} -> ${upgradeStatus.latest_version ?? "unknown"}`
        : `You are already on ${upgradeStatus.current_version ?? "the latest release"}`;

  const upgradeBody = !upgradeStatus
    ? "Load runtime config to inspect the installed release and GitHub release status."
    : !upgradeStatus.supported
      ? upgradeStatus.reason ?? "Self-upgrade only works when chaos-bot is started from an installed release bundle."
      : upgradeStatus.upgrade_available
        ? `The Web UI can download and install ${upgradeStatus.latest_version ?? "the latest release"} now. A launcher restart is still required after install.`
        : upgradeStatus.reason ?? "No newer GitHub release is available for this installed bundle.";

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
          <button type="button" className="ghost-btn" onClick={() => void applySearchConfig()} disabled={busy || !state}>
            Save Search Settings
          </button>
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
          <h3>Web Upgrade</h3>
          <button type="button" className="ghost-btn" onClick={() => void refreshUpgradeStatus()} disabled={busy}>
            Refresh Upgrade
          </button>
        </div>

        <div className={`upgrade-card ${upgradeStatus?.upgrade_available ? "available" : ""}`}>
          <div className="upgrade-copy">
            <p className="upgrade-eyebrow">GitHub release installer</p>
            <h4>{upgradeHeadline}</h4>
            <p className="upgrade-body">{upgradeBody}</p>
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

          {upgradeStatus?.repository ? <p className="upgrade-detail">repository: {upgradeStatus.repository}</p> : null}
          {upgradeStatus?.install_prefix ? <p className="upgrade-detail">prefix: {upgradeStatus.install_prefix}</p> : null}
          {upgradeStatus?.download_url ? <p className="upgrade-detail">bundle: {upgradeStatus.download_url}</p> : null}

          <div className="config-actions">
            <button
              type="button"
              data-testid="upgrade-apply-button"
              onClick={() => void applyUpgrade()}
              disabled={busy || !upgradeStatus?.supported || !upgradeStatus.upgrade_available}
            >
              {action === "upgrade" ? "Installing..." : "Install Latest Release"}
            </button>
            <button
              type="button"
              className="ghost-btn"
              data-testid="upgrade-relaunch-button"
              onClick={() => void relaunchUpgrade()}
              disabled={busy || !upgradeStatus?.supported || !lastUpgradeResult?.relaunch_required}
            >
              {action === "relaunch" ? "Restarting..." : "Restart Now"}
            </button>
          </div>
        </div>

        <div className="config-meta">
          <p>
            install type <strong>{upgradeStatus?.supported ? "bundle" : "dev/runtime"}</strong>
          </p>
          <p>
            relaunch <strong>required after install</strong>
          </p>
          <p>
            action <strong>web ui</strong>
          </p>
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
