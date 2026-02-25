import { FormEvent, useEffect, useMemo, useState } from "react";
import { ConfigPanel } from "./components/ConfigPanel";
import { ConversationPanel } from "./components/ConversationPanel";
import { EventTimeline } from "./components/EventTimeline";
import { MobilePaneTabs, type MobilePane } from "./components/MobilePaneTabs";
import { SessionRail } from "./components/SessionRail";
import type { ChatStreamEnvelope, RuntimeError, SessionState } from "./contracts/protocol";
import { useLayoutAdapter } from "./layout/adapter";
import { createRuntimeAdapter } from "./runtime";

type StreamLog = {
  id: string;
  summary: string;
};

type DesktopSidePane = "events" | "config";

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

export default function App() {
  const runtime = useMemo(() => createRuntimeAdapter(), []);
  const layout = useLayoutAdapter();

  const [baseUrl, setBaseUrl] = useState(resolveDefaultBaseUrl);
  const [health, setHealth] = useState("pending");
  const [sessions, setSessions] = useState<SessionState[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | undefined>();
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [streamLogs, setStreamLogs] = useState<StreamLog[]>([]);
  const [runtimeError, setRuntimeError] = useState<RuntimeError | undefined>();
  const [mobilePane, setMobilePane] = useState<MobilePane>("chat");
  const [desktopSidePane, setDesktopSidePane] = useState<DesktopSidePane>("events");

  const activeSession = sessions.find((session) => session.id === activeSessionId);

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

  async function handleCreateSession() {
    try {
      const session = await runtime.createSession(baseUrl);
      setActiveSessionId(session.id);
      await reloadSessions();
      setRuntimeError(undefined);
      pushLog(`[session.create] ${session.id}`);
      if (layout.isMobile) {
        setMobilePane("chat");
      }
    } catch (error) {
      setRuntimeError(asRuntimeError(error));
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
      setRuntimeError(asRuntimeError(error));
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

  async function handleSend(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const text = draft.trim();
    if (!text || sending) {
      return;
    }

    setDraft("");
    setSending(true);
    setRuntimeError(undefined);
    pushLog(`[request] ${text}`);

    try {
      await runtime.chatStream(
        baseUrl,
        { session_id: activeSessionId, message: text },
        handleStreamEvent,
        (error) => setRuntimeError(error),
      );
      await reloadSessions();
    } catch (error) {
      setRuntimeError(asRuntimeError(error));
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

        {(layout.isDesktop || mobilePane === "chat" || mobilePane === "events" || mobilePane === "config") && (
          <main className={`chat-main ${layout.mode}`}>
            {layout.isDesktop ? (
              <>
                <ConversationPanel
                  session={activeSession}
                  draft={draft}
                  sending={sending}
                  onDraftChange={setDraft}
                  onSubmit={(evt) => void handleSend(evt)}
                  onDeleteSession={() => void handleDeleteSession()}
                />

                <aside className="side-pane">
                  <nav className="pane-tabs" aria-label="Desktop side panes">
                    <button
                      type="button"
                      className={desktopSidePane === "events" ? "active" : ""}
                      onClick={() => setDesktopSidePane("events")}
                    >
                      Events
                    </button>
                    <button
                      type="button"
                      className={desktopSidePane === "config" ? "active" : ""}
                      onClick={() => setDesktopSidePane("config")}
                    >
                      Config
                    </button>
                  </nav>

                  {desktopSidePane === "events" ? (
                    <EventTimeline streamLogs={streamLogs} runtimeError={runtimeError} />
                  ) : (
                    <ConfigPanel
                      runtime={runtime}
                      baseUrl={baseUrl}
                      onLog={pushLog}
                      onRuntimeError={setRuntimeError}
                    />
                  )}
                </aside>
              </>
            ) : null}

            {layout.isMobile && mobilePane === "chat" ? (
              <ConversationPanel
                session={activeSession}
                draft={draft}
                sending={sending}
                onDraftChange={setDraft}
                onSubmit={(evt) => void handleSend(evt)}
                onDeleteSession={() => void handleDeleteSession()}
              />
            ) : null}

            {layout.isMobile && mobilePane === "events" ? (
              <EventTimeline streamLogs={streamLogs} runtimeError={runtimeError} />
            ) : null}

            {layout.isMobile && mobilePane === "config" ? (
              <ConfigPanel
                runtime={runtime}
                baseUrl={baseUrl}
                compact
                onLog={pushLog}
                onRuntimeError={setRuntimeError}
              />
            ) : null}
          </main>
        )}
      </section>
    </div>
  );
}
