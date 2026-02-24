import type { SessionState } from "../contracts/protocol";

type SessionRailProps = {
  sessions: SessionState[];
  activeSessionId?: string;
  baseUrl: string;
  health: string;
  transport: string;
  compact: boolean;
  onBaseUrlChange: (value: string) => void;
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
  baseUrl,
  health,
  transport,
  compact,
  onBaseUrlChange,
  onSelectSession,
  onCreateSession,
  onRefresh,
}: SessionRailProps) {
  return (
    <aside className={`rail ${compact ? "compact" : ""}`}>
      <div className="rail-head">
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

      <div className="runtime-meta">
        <p>
          transport <strong>{transport}</strong>
        </p>
        <p>
          health <strong>{health}</strong>
        </p>
      </div>

      <label className="base-url">
        <span>Backend URL</span>
        <input value={baseUrl} onChange={(evt) => onBaseUrlChange(evt.target.value)} />
      </label>

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
    </aside>
  );
}
