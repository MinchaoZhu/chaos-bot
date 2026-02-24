import type { FormEvent } from "react";
import type { SessionState } from "../contracts/protocol";

type ConversationPanelProps = {
  session?: SessionState;
  draft: string;
  sending: boolean;
  onDraftChange: (value: string) => void;
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onDeleteSession: () => void;
};

export function ConversationPanel({
  session,
  draft,
  sending,
  onDraftChange,
  onSubmit,
  onDeleteSession,
}: ConversationPanelProps) {
  return (
    <section className="panel chat-panel">
      <div className="panel-head">
        <h2>Conversation</h2>
        <button
          type="button"
          className="danger-btn"
          onClick={onDeleteSession}
          disabled={!session}
          aria-disabled={!session}
        >
          Delete Session
        </button>
      </div>

      <div className="messages">
        {session?.messages
          .filter((item) => item.role !== "system")
          .map((item, idx) => (
            <article className={`msg ${item.role}`} key={`${item.role}-${idx}`}>
              <p className="role">{item.role}</p>
              <p>{item.content ?? ""}</p>
            </article>
          ))}
      </div>

      <form className="composer" onSubmit={onSubmit}>
        <input
          value={draft}
          onChange={(evt) => onDraftChange(evt.target.value)}
          placeholder="Type prompt and stream through runtime contract"
          disabled={sending}
        />
        <button type="submit" disabled={sending || !draft.trim()}>
          {sending ? "Streaming..." : "Send"}
        </button>
      </form>
    </section>
  );
}
