import { FormEvent, useEffect, useMemo, useState } from "react";
import {
  buildCompactSummary,
  formatHelpLines,
  getSlashCommandHints,
  modelsForProvider,
  parseSlashCommand,
  type ParsedSlashCommand,
  type SlashCommandSpec,
} from "./commands/slash";
import { AboutPanel } from "./components/AboutPanel";
import { ConfigPanel } from "./components/ConfigPanel";
import { ConversationPanel } from "./components/ConversationPanel";
import { EventTimeline } from "./components/EventTimeline";
import { PrimaryTabs, type PrimaryPane } from "./components/MobilePaneTabs";
import { SessionRail } from "./components/SessionRail";
import { SkillsPanel } from "./components/SkillsPanel";
import type {
  ChannelStatusResponse,
  ChatStreamEnvelope,
  ConfigStateResponse,
  RuntimeError,
  SessionState,
} from "./contracts/protocol";
import { useLayoutAdapter } from "./layout/adapter";
import { createRuntimeAdapter } from "./runtime";

type StreamLog = {
  id: string;
  summary: string;
};

function resolveDefaultBaseUrl(): string {
  if (typeof window === "undefined") {
    return "http://127.0.0.1:3000";
  }

  if (window.__TAURI_INTERNALS__) {
    return "http://127.0.0.1:3000";
  }

  if (window.location.protocol === "http:" || window.location.protocol === "https:") {
    return window.location.origin;
  }

  return "http://127.0.0.1:3000";
}

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

function toRuntimeError(error: unknown): RuntimeError {
  if (error && typeof error === "object") {
    const maybeCode = (error as { code?: string }).code;
    const maybeMessage = (error as { message?: string }).message;
    if (typeof maybeCode === "string" && typeof maybeMessage === "string") {
      return { code: maybeCode as RuntimeError["code"], message: maybeMessage };
    }
  }
  return { code: "UNKNOWN", message: String(error) };
}

function withUpdatedModel(config: ConfigStateResponse["running"], model: string): ConfigStateResponse["running"] {
  return {
    ...config,
    llm: { ...(config.llm ?? {}), model },
  };
}

export default function App() {
  const runtime = useMemo(() => createRuntimeAdapter(), []);
  const layout = useLayoutAdapter();

  const [baseUrl, setBaseUrl] = useState(resolveDefaultBaseUrl);
  const [health, setHealth] = useState("pending");
  const [channelStatus, setChannelStatus] = useState<ChannelStatusResponse | undefined>();
  const [sessions, setSessions] = useState<SessionState[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [streamLogs, setStreamLogs] = useState<StreamLog[]>([]);
  const [runtimeError, setRuntimeError] = useState<RuntimeError | undefined>();
  const [activePane, setActivePane] = useState<PrimaryPane>("conversation");

  const activeSession = sessions.find((session) => session.id === activeSessionId);
  const commandHints = useMemo(() => getSlashCommandHints(draft), [draft]);

  function pushLog(summary: string) {
    setStreamLogs((prev) => [{ id: `${Date.now()}-${prev.length}`, summary }, ...prev].slice(0, 18));
  }

  async function refreshHealth() {
    const response = await runtime.health(baseUrl);
    setHealth(`${response.status} @ ${response.now}`);
    const channels = await runtime.channelStatus(baseUrl);
    setChannelStatus(channels);
  }

  async function reloadSessions() {
    const list = await runtime.listSessions(baseUrl);
    setSessions(list);
    setActiveSessionId((prev) => {
      if (prev && list.some((item) => item.id === prev)) {
        return prev;
      }
      return list[0]?.id;
    });
  }

  async function createAndSelectSession(): Promise<SessionState> {
    const session = await runtime.createSession(baseUrl);
    await reloadSessions();
    setActiveSessionId(session.id);
    setActivePane("conversation");
    return session;
  }

  async function handleCreateSession() {
    try {
      const session = await createAndSelectSession();
      setRuntimeError(undefined);
      pushLog(`[session.create] ${session.id}`);
    } catch (error) {
      setRuntimeError(toRuntimeError(error));
    }
  }

  async function handleDeleteSession() {
    if (!activeSessionId) {
      return;
    }

    try {
      await runtime.deleteSession(baseUrl, activeSessionId);
      pushLog(`[session.delete] ${activeSessionId}`);
      await reloadSessions();
      setRuntimeError(undefined);
    } catch (error) {
      setRuntimeError(toRuntimeError(error));
    }
  }

  async function handleSelectSession(sessionId: string) {
    try {
      const session = await runtime.getSession(baseUrl, sessionId);
      setSessions((prev) => {
        const next = prev.map((item) => (item.id === session.id ? session : item));
        return next.some((item) => item.id === session.id) ? next : [session, ...next];
      });
      setActiveSessionId(session.id);
      setActivePane("conversation");
      setRuntimeError(undefined);
      pushLog(`[session.select] ${session.id}`);
    } catch (error) {
      setRuntimeError(toRuntimeError(error));
    }
  }

  function handleStreamEvent(event: ChatStreamEnvelope) {
    if (event.event === "session") {
      const payload = event.data as { session_id?: string };
      if (payload.session_id) {
        setActiveSessionId(payload.session_id);
      }
    }

    pushLog(`[${event.event}] ${asText(event.data)}`);
  }

  async function runChatMessage(message: string, sessionId?: string) {
    pushLog(`[request] ${message}`);
    await runtime.chatStream(
      baseUrl,
      { session_id: sessionId, message },
      handleStreamEvent,
      (error) => setRuntimeError(error),
    );
    await reloadSessions();
  }

  async function runShowModel() {
    const state = await runtime.getConfig(baseUrl);
    const provider = state.running.llm.provider ?? "openai";
    const model = state.running.llm.model ?? "gpt-4o-mini";
    pushLog(`[command.model] provider=${provider} model=${model}`);
  }

  async function runModelsCommand(args: string[]) {
    const state = await runtime.getConfig(baseUrl);
    const provider = state.running.llm.provider ?? "openai";
    const currentModel = state.running.llm.model ?? "gpt-4o-mini";
    const allowedModels = modelsForProvider(provider);

    if (args.length === 0) {
      pushLog(`[command.models] provider=${provider} current=${currentModel}`);
      pushLog(`[command.models] options: ${allowedModels.join(", ")}`);
      return;
    }

    if (args[0] !== "set" || !args[1]) {
      pushLog("[command.error] usage: /models set <model_id>");
      return;
    }

    const target = args[1];
    if (!allowedModels.includes(target)) {
      pushLog(`[command.error] unsupported model for ${provider}: ${target}`);
      pushLog(`[command.models] allowed: ${allowedModels.join(", ")}`);
      return;
    }

    if (target === currentModel) {
      pushLog(`[command.models] model already active: ${target}`);
      return;
    }

    const nextConfig = withUpdatedModel(state.running, target);
    await runtime.applyConfig(baseUrl, { config: nextConfig });
    pushLog(`[command.models] switched model ${currentModel} -> ${target}`);
  }

  async function runCompactCommand() {
    const sourceSession = activeSession;
    if (!sourceSession) {
      const created = await createAndSelectSession();
      pushLog(`[command.compact] no active session; created ${created.id}`);
      return;
    }

    const summary = buildCompactSummary(sourceSession);
    const destination = await createAndSelectSession();
    const compactPrompt = [
      "You are receiving compacted context from an earlier session.",
      "Use it as background memory for all future replies in this session.",
      "If understood, reply exactly with: Context loaded.",
      "",
      summary,
    ].join("\n");

    await runChatMessage(compactPrompt, destination.id);
    pushLog(`[command.compact] migrated ${sourceSession.id.slice(0, 8)} -> ${destination.id.slice(0, 8)}`);
  }

  async function executeSlashCommand(command: ParsedSlashCommand) {
    switch (command.name) {
      case "help":
        pushLog("[command.help] available commands:");
        formatHelpLines().forEach((line) => pushLog(`[command.help] ${line}`));
        return;
      case "clear":
        setDraft("");
        pushLog("[command.clear] draft cleared");
        return;
      case "sessions":
        setActivePane("sessions");
        pushLog("[command.sessions] switched to sessions pane");
        return;
      case "new": {
        const session = await createAndSelectSession();
        pushLog(`[command.new] created ${session.id}`);
        return;
      }
      case "model":
        await runShowModel();
        return;
      case "models":
        await runModelsCommand(command.args);
        return;
      case "compact":
        await runCompactCommand();
        return;
      default:
        pushLog(`[command.error] unsupported command: ${command.name}`);
    }
  }

  function handleSelectCommandHint(hint: SlashCommandSpec) {
    setDraft(hint.completion);
  }

  async function handleSend(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (sending) {
      return;
    }

    const text = draft.trim();
    if (!text) {
      return;
    }

    const parsed = parseSlashCommand(text);
    if (parsed.kind === "error") {
      pushLog(`[command.error] ${parsed.message}`);
      return;
    }

    setDraft("");
    setSending(true);
    setRuntimeError(undefined);
    try {
      if (parsed.kind === "not_command" || parsed.kind === "escaped") {
        if (!parsed.text) {
          return;
        }
        await runChatMessage(parsed.text, activeSessionId);
        return;
      }

      await executeSlashCommand(parsed.command);
    } catch (error) {
      const runtimeFailure = toRuntimeError(error);
      setRuntimeError(runtimeFailure);
      pushLog(`[command.error] ${runtimeFailure.message}`);
    } finally {
      setSending(false);
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        await refreshHealth();
        await reloadSessions();
        if (!cancelled) {
          setRuntimeError(undefined);
        }
      } catch (error) {
        if (!cancelled) {
          setRuntimeError(asRuntimeError(error));
          setHealth("unreachable");
        }
      }
    }

    void bootstrap();

    return () => {
      cancelled = true;
    };
  }, [runtime, baseUrl]);

  return (
    <div className="page">
      <header className="hero">
        <p className="eyebrow">Tauri v2 + React Runtime Shell</p>
        <h1>chaos-bot multi-platform UI foundation</h1>
        <p className="hero-note">One contract, two form factors: desktop landscape and mobile portrait.</p>
      </header>

      <PrimaryTabs activePane={activePane} onChange={setActivePane} />

      <section className={`workspace ${layout.mode}`}>
        {activePane === "conversation" ? (
          <ConversationPanel
            session={activeSession}
            draft={draft}
            sending={sending}
            commandHints={commandHints}
            onDraftChange={setDraft}
            onSubmit={(evt) => void handleSend(evt)}
            onSelectCommandHint={handleSelectCommandHint}
            onDeleteSession={() => void handleDeleteSession()}
          />
        ) : null}

        {activePane === "sessions" ? (
          <SessionRail
            sessions={sessions}
            activeSessionId={activeSessionId}
            compact={layout.isMobile}
            onSelectSession={(sessionId) => void handleSelectSession(sessionId)}
            onCreateSession={() => void handleCreateSession()}
            onRefresh={() => {
              void refreshHealth();
              void reloadSessions();
            }}
          />
        ) : null}

        {activePane === "events" ? (
          <EventTimeline streamLogs={streamLogs} runtimeError={runtimeError} />
        ) : null}

        {activePane === "config" ? (
          <ConfigPanel
            runtime={runtime}
            baseUrl={baseUrl}
            health={health}
            transport={runtime.source}
            channelStatus={channelStatus}
            compact={layout.isMobile}
            onBaseUrlChange={setBaseUrl}
            onLog={pushLog}
            onRuntimeError={setRuntimeError}
          />
        ) : null}

        {activePane === "skills" ? (
          <SkillsPanel runtime={runtime} baseUrl={baseUrl} onLog={pushLog} />
        ) : null}

        {activePane === "about" ? (
          <AboutPanel runtime={runtime} baseUrl={baseUrl} transport={runtime.source} onLog={pushLog} onRuntimeError={setRuntimeError} />
        ) : null}
      </section>
    </div>
  );
}
