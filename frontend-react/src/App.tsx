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
import { ConversationPanel } from "./components/ConversationPanel";
import { EventTimeline } from "./components/EventTimeline";
import { MobilePaneTabs, type MobilePane } from "./components/MobilePaneTabs";
import { SessionRail } from "./components/SessionRail";
import type { AgentFileConfig, ChatStreamEnvelope, RuntimeError, SessionState } from "./contracts/protocol";
import { useLayoutAdapter } from "./layout/adapter";
import { createRuntimeAdapter } from "./runtime";

type StreamLog = {
  id: string;
  summary: string;
};

function asText(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }
  return JSON.stringify(value);
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

function withUpdatedModel(config: AgentFileConfig, model: string): AgentFileConfig {
  return {
    workspace: config.workspace,
    server: { ...(config.server ?? {}) },
    llm: { ...(config.llm ?? {}), model },
    logging: { ...(config.logging ?? {}) },
    secrets: { ...(config.secrets ?? {}) },
  };
}

export default function App() {
  const runtime = useMemo(() => createRuntimeAdapter(), []);
  const layout = useLayoutAdapter();

  const [baseUrl, setBaseUrl] = useState("http://127.0.0.1:3000");
  const [health, setHealth] = useState("pending");
  const [sessions, setSessions] = useState<SessionState[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [streamLogs, setStreamLogs] = useState<StreamLog[]>([]);
  const [runtimeError, setRuntimeError] = useState<RuntimeError | undefined>();
  const [mobilePane, setMobilePane] = useState<MobilePane>("chat");

  const activeSession = sessions.find((session) => session.id === activeSessionId);
  const commandHints = useMemo(() => getSlashCommandHints(draft), [draft]);

  function pushLog(summary: string) {
    setStreamLogs((prev) => [{ id: `${Date.now()}-${prev.length}`, summary }, ...prev].slice(0, 18));
  }

  async function refreshHealth() {
    const response = await runtime.health(baseUrl);
    setHealth(`${response.status} @ ${response.now}`);
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
    setActiveSessionId(session.id);
    await reloadSessions();
    if (layout.isMobile) {
      setMobilePane("chat");
    }
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
    await runtime.applyConfig(baseUrl, nextConfig);
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
        if (layout.isMobile) {
          setMobilePane("sessions");
          pushLog("[command.sessions] switched to sessions pane");
        } else {
          pushLog("[command.sessions] desktop already shows sessions rail");
        }
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
          setRuntimeError({ code: "UNKNOWN", message: String(error) });
          setHealth("unreachable");
        }
      }
    }

    void bootstrap();

    return () => {
      cancelled = true;
    };
  }, [runtime, baseUrl]);

  useEffect(() => {
    if (layout.isDesktop) {
      setMobilePane("chat");
    }
  }, [layout.isDesktop]);

  return (
    <div className="page">
      <header className="hero">
        <p className="eyebrow">Tauri v2 + React Runtime Shell</p>
        <h1>chaos-bot multi-platform UI foundation</h1>
        <p className="hero-note">One contract, two form factors: desktop landscape and mobile portrait.</p>
      </header>

      {layout.isMobile ? <MobilePaneTabs activePane={mobilePane} onChange={setMobilePane} /> : null}

      <section className={`workspace ${layout.mode}`}>
        {(layout.isDesktop || mobilePane === "sessions") && (
          <SessionRail
            sessions={sessions}
            activeSessionId={activeSessionId}
            baseUrl={baseUrl}
            health={health}
            transport={runtime.source}
            compact={layout.isMobile}
            onBaseUrlChange={setBaseUrl}
            onSelectSession={setActiveSessionId}
            onCreateSession={() => void handleCreateSession()}
            onRefresh={() => {
              void refreshHealth();
              void reloadSessions();
            }}
          />
        )}

        {(layout.isDesktop || mobilePane === "chat" || mobilePane === "events") && (
          <main className="chat-main">
            {(layout.isDesktop || mobilePane === "chat") && (
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
            )}

            {(layout.isDesktop || mobilePane === "events") && (
              <EventTimeline streamLogs={streamLogs} runtimeError={runtimeError} />
            )}
          </main>
        )}
      </section>
    </div>
  );
}
