import type { RuntimeError } from "../contracts/protocol";

type StreamLog = {
  id: string;
  summary: string;
};

type EventTimelineProps = {
  streamLogs: StreamLog[];
  runtimeError?: RuntimeError;
};

export function EventTimeline({ streamLogs, runtimeError }: EventTimelineProps) {
  return (
    <section className="panel event-panel">
      <h2>Stream Events</h2>
      {runtimeError ? (
        <div className="runtime-error">
          {runtimeError.code}: {runtimeError.message}
        </div>
      ) : null}
      <ul>
        {streamLogs.map((entry) => (
          <li key={entry.id}>{entry.summary}</li>
        ))}
      </ul>
    </section>
  );
}
