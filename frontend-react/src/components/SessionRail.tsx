import type { SessionState } from "../contracts/protocol";

type SessionRailProps = {
  sessions: SessionState[];
  activeSessionId?: string;
  compact: boolean;
  onSelectSession: (sessionId: string) => void;
  onCreateSession: () => void;
  onRefresh: () => void;
};

function messagePreview(session: SessionState): string {
  const latest = [...session.messages].reverse().find((msg) => msg.role !== "system");
  return latest?.content ?? "(empty)";
}

export function SessionRail({
  sessions,
  activeSessionId,
  compact,
  onSelectSession,
  onCreateSession,
  onRefresh,
}: SessionRailProps) {
  return (
    <section className={`panel sessions-panel ${compact ? "compact" : ""}`}>
      <div className="panel-head">
        <h2>Sessions</h2>
        <div className="rail-actions">
          <button type="button" onClick={onRefresh} className="ghost-btn">
            Refresh
          </button>
          <button type="button" onClick={onCreateSession}>
            New
          </button>
        </div>
      </div>

      <p className="sessions-note">
        Select a previous session to load its conversation history. New sessions open directly in the chat view.
      </p>

      <ul className="session-list">
        {sessions.map((session) => (
          <li key={session.id}>
            <button
              type="button"
              className={session.id === activeSessionId ? "active" : ""}
              onClick={() => onSelectSession(session.id)}
            >
              <strong>{session.id.slice(0, 8)}</strong>
              <span>{messagePreview(session)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
