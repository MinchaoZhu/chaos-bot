import type { ChannelStatusResponse, SessionState } from "../contracts/protocol";

type TelegramConnectorDraft = {
  enabled: boolean;
  polling: boolean;
  apiBaseUrl: string;
  webhookSecret: string;
  webhookBaseUrl: string;
  botToken: string;
};

type SessionRailProps = {
  sessions: SessionState[];
  activeSessionId?: string;
  baseUrl: string;
  health: string;
  channelStatus?: ChannelStatusResponse;
  telegramDraft: TelegramConnectorDraft;
  configBusy: boolean;
  configNotice?: string;
  transport: string;
  compact: boolean;
  onBaseUrlChange: (value: string) => void;
  onTelegramDraftChange: (patch: Partial<TelegramConnectorDraft>) => void;
  onApplyConnectorConfig: () => void;
  onResetConfig: () => void;
  onRestartRuntime: () => void;
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
  channelStatus,
  telegramDraft,
  configBusy,
  configNotice,
  transport,
  compact,
  onBaseUrlChange,
  onTelegramDraftChange,
  onApplyConnectorConfig,
  onResetConfig,
  onRestartRuntime,
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

      <div className="channel-meta">
        <p>
          channels <strong>{channelStatus?.enabled_channels.join(", ") || "none"}</strong>
        </p>
        <p>
          telegram{" "}
          <strong>
            {channelStatus?.telegram.enabled
              ? channelStatus.telegram.polling
                ? "enabled (polling)"
                : "enabled (webhook)"
              : "disabled"}
          </strong>
        </p>
      </div>

      <label className="base-url">
        <span>Backend URL</span>
        <input value={baseUrl} onChange={(evt) => onBaseUrlChange(evt.target.value)} />
      </label>

      <section className="connector-config">
        <h3>Telegram connector</h3>
        <div className="connector-toggles">
          <label>
            <input
              type="checkbox"
              checked={telegramDraft.enabled}
              onChange={(evt) => onTelegramDraftChange({ enabled: evt.target.checked })}
            />
            <span>enabled</span>
          </label>
          <label>
            <input
              type="checkbox"
              checked={telegramDraft.polling}
              onChange={(evt) => onTelegramDraftChange({ polling: evt.target.checked })}
            />
            <span>polling mode</span>
          </label>
        </div>
        <label>
          <span>api base url</span>
          <input
            value={telegramDraft.apiBaseUrl}
            onChange={(evt) => onTelegramDraftChange({ apiBaseUrl: evt.target.value })}
            placeholder="https://api.telegram.org"
          />
        </label>
        <label>
          <span>webhook secret</span>
          <input
            value={telegramDraft.webhookSecret}
            onChange={(evt) => onTelegramDraftChange({ webhookSecret: evt.target.value })}
            placeholder="x-telegram-bot-api-secret-token"
          />
        </label>
        <label>
          <span>webhook base url</span>
          <input
            value={telegramDraft.webhookBaseUrl}
            onChange={(evt) => onTelegramDraftChange({ webhookBaseUrl: evt.target.value })}
            placeholder="https://example.com"
          />
        </label>
        <label>
          <span>bot token</span>
          <input
            value={telegramDraft.botToken}
            onChange={(evt) => onTelegramDraftChange({ botToken: evt.target.value })}
            placeholder="BotFather token"
          />
        </label>
        <div className="connector-actions">
          <button type="button" onClick={onApplyConnectorConfig} disabled={configBusy}>
            Apply
          </button>
          <button type="button" className="ghost-btn" onClick={onResetConfig} disabled={configBusy}>
            Reset
          </button>
          <button type="button" className="ghost-btn" onClick={onRestartRuntime} disabled={configBusy}>
            Restart
          </button>
        </div>
        {configNotice ? <p className="config-notice">{configNotice}</p> : null}
      </section>

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
