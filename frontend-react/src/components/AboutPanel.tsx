import { useEffect, useState } from "react";
import packageJson from "../../package.json";
import type { RuntimeError, UpgradeStatusResponse } from "../contracts/protocol";
import type { RuntimeAdapter } from "../runtime/adapter";

type AboutPanelProps = {
  runtime: RuntimeAdapter;
  baseUrl: string;
  transport: string;
  onLog: (summary: string) => void;
  onRuntimeError: (error: RuntimeError | undefined) => void;
};

const REPOSITORY_URL = "https://github.com/MinchaoZhu/chaos-bot";

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

export function AboutPanel({ runtime, baseUrl, transport, onLog, onRuntimeError }: AboutPanelProps) {
  const [status, setStatus] = useState<UpgradeStatusResponse>();
  const [notice, setNotice] = useState("");
  const [action, setAction] = useState<"refresh" | "upgrade" | undefined>();

  async function refreshUpgradeStatus() {
    setAction("refresh");
    try {
      const next = await runtime.getUpgradeStatus(baseUrl);
      setStatus(next);
      setNotice(
        next.supported
          ? next.upgrade_available
            ? `Upgrade available: ${next.current_version ?? "unknown"} -> ${next.latest_version ?? "unknown"}`
            : `Up to date: ${next.current_version ?? packageJson.version}`
          : `Upgrade unavailable: ${next.reason ?? "unsupported runtime"}`,
      );
      onRuntimeError(undefined);
      onLog(`[upgrade.status] available=${next.upgrade_available}`);
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setNotice(`upgrade status failed: ${runtimeError.message}`);
    } finally {
      setAction(undefined);
    }
  }

  async function applyUpgrade() {
    setAction("upgrade");
    try {
      const response = await runtime.applyUpgrade(baseUrl);
      setStatus(response.status);
      setNotice(response.message);
      onRuntimeError(undefined);
      onLog(
        `[upgrade.apply] action=${response.action} current=${response.current_version ?? "-"} target=${response.target_version ?? "-"}`,
      );
    } catch (error) {
      const runtimeError = asRuntimeError(error);
      onRuntimeError(runtimeError);
      setNotice(`upgrade failed: ${runtimeError.message}`);
    } finally {
      setAction(undefined);
    }
  }

  useEffect(() => {
    void refreshUpgradeStatus();
    // Re-load when runtime target changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [baseUrl, runtime]);

  return (
    <section className="panel about-panel">
      <div className="panel-head">
        <h2>About</h2>
        <button type="button" className="ghost-btn" onClick={() => void refreshUpgradeStatus()} disabled={Boolean(action)}>
          {action === "refresh" ? "Refreshing..." : "Refresh"}
        </button>
      </div>

      <div className="about-grid">
        <article className="about-card">
          <p className="upgrade-eyebrow">Project</p>
          <h3>chaos-bot runtime shell</h3>
          <p className="about-copy">
            Desktop and web UI for the chaos-bot runtime, including chat, sessions, skills, config editing, and release upgrade flows.
          </p>
        </article>

        <article className="about-card">
          <p className="upgrade-eyebrow">Repository</p>
          <h3>MinchaoZhu/chaos-bot</h3>
          <p className="about-copy">
            Maintained from the main GitHub repository for the project.
          </p>
          <a className="about-link" href={REPOSITORY_URL} target="_blank" rel="noreferrer">
            {REPOSITORY_URL}
          </a>
        </article>
      </div>

      <div className="config-meta">
        <p>
          app version <strong>{packageJson.version}</strong>
        </p>
        <p>
          installed <strong>{status?.current_version ?? packageJson.version}</strong>
        </p>
        <p>
          transport <strong>{transport}</strong>
        </p>
      </div>

      <div className={`upgrade-card ${status?.upgrade_available ? "available" : ""}`}>
        <div className="upgrade-copy">
          <p className="upgrade-eyebrow">Release upgrade</p>
          <h4>
            {!status
              ? "Checking installed release..."
              : !status.supported
                ? "Web UI upgrade is unavailable in this runtime"
                : status.upgrade_available
                  ? `Upgrade ready: ${status.current_version ?? "unknown"} -> ${status.latest_version ?? "unknown"}`
                  : `You are already on ${status.current_version ?? packageJson.version}`}
          </h4>
          <p className="upgrade-body">
            {!status
              ? "Load runtime metadata to inspect the installed release and latest GitHub release."
              : !status.supported
                ? status.reason ?? "Self-upgrade only works when chaos-bot is started from an installed release bundle."
                : status.upgrade_available
                  ? `The UI can download and install ${status.latest_version ?? "the latest release"} now. A relaunch is still required afterwards.`
                  : status.reason ?? "No newer GitHub release is available for this installed bundle."}
          </p>
        </div>

        <div className="config-meta">
          <p>
            latest <strong>{status?.latest_version ?? "-"}</strong>
          </p>
          <p>
            available <strong>{status?.upgrade_available ? "yes" : "no"}</strong>
          </p>
          <p>
            install type <strong>{status?.supported ? "bundle" : "dev/runtime"}</strong>
          </p>
        </div>

        {status?.repository ? <p className="upgrade-detail">repository: {status.repository}</p> : null}
        {status?.latest_release_url ? <p className="upgrade-detail">release: {status.latest_release_url}</p> : null}
        {status?.download_url ? <p className="upgrade-detail">bundle: {status.download_url}</p> : null}

        <div className="config-actions">
          <button
            type="button"
            data-testid="upgrade-apply-button"
            onClick={() => void applyUpgrade()}
            disabled={Boolean(action) || !status?.supported || !status.upgrade_available}
          >
            {action === "upgrade" ? "Installing..." : "Install Latest Release"}
          </button>
        </div>
      </div>

      {notice ? <p className="config-status">{notice}</p> : null}
    </section>
  );
}
