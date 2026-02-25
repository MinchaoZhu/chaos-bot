import { useEffect, useState } from "react";
import type {
  ConfigStateResponse,
  RuntimeError,
} from "../contracts/protocol";
import type { RuntimeAdapter } from "../runtime/adapter";

type ConfigPanelProps = {
  runtime: RuntimeAdapter;
  baseUrl: string;
  compact?: boolean;
  onLog: (summary: string) => void;
  onRuntimeError: (error: RuntimeError | undefined) => void;
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

export function ConfigPanel({
  runtime,
  baseUrl,
  compact,
  onLog,
  onRuntimeError,
}: ConfigPanelProps) {
  const [state, setState] = useState<ConfigStateResponse>();
  const [raw, setRaw] = useState("");
  const [status, setStatus] = useState("");
  const [loading, setLoading] = useState(false);
  const [action, setAction] = useState<"apply" | "reset" | "restart" | undefined>();

  const busy = loading || Boolean(action);

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
