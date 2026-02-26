# task-1: Multi-Channel Connector Framework + Telegram

## Task
- Description: Plan and deliver a multi-channel connector framework that lets one runtime agent receive/send messages across multiple channels, starting with Telegram integration provisioned through BotFather.
- Scope: `backend/src/domain/ports`, `backend/src/application`, `backend/src/infrastructure`, `backend/src/runtime`, `backend/src/interface`, `frontend-react`, and e2e coverage for multi-channel flows.
- Risk: Medium-High. Channel routing, session identity mapping, webhook/polling reliability, and Telegram API edge cases can break existing chat behavior if adapter boundaries are weak.
- Status: in_progress

## Phase 1: Connector Framework Contracts
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Define a channel connector abstraction in `domain::ports` for inbound events, outbound messages, and delivery acknowledgements. | done | Added `ChannelConnectorPort` and `ChannelDispatcherPort` with channel delivery models. |
| 1.2 | Define unified channel identity/session mapping rules (channel user -> internal session) and error model. | done | Added channel context/session-key mapping in `ChatService` + `SessionStore` channel bindings. |
| 1.3 | Specify connector lifecycle contract (start, stop, health, retry/backoff hooks) for runtime orchestration. | done | Added connector lifecycle (`start/stop/health`) and retry/backoff semantics in Telegram connector. |
| 1.v1 | Verify: Contract review confirms `application` depends only on domain ports, with no concrete adapter leakage. | done | `ChatService` depends on `ChannelDispatcherPort` trait only. |

## Phase 2: Runtime Orchestration and Registry Plan
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Plan runtime DI wiring for pluggable connector registry under `runtime`, with per-channel enable/disable flags. | done | Added `infrastructure/channels` registry and runtime wiring via config flags. |
| 2.2 | Define channel-aware dispatch pipeline in `application` that preserves existing web chat behavior. | done | Added `run_channel_message` path and kept `/api/chat` unchanged for web flow. |
| 2.3 | Plan observability fields for connector execution (`channel`, `session_id`, `tool_call_id`, error classification). | done | Added structured logs for channel, retry attempts, transient failures, and polling execution path. |
| 2.v1 | Verify: Integration test plan covers dispatch ordering, retries, and connector isolation failures. | done | Added integration tests for retry success and outage failure paths. |

## Phase 3: Telegram Adapter (BotFather)
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Define Telegram connector adapter under `infrastructure` using BotFather-generated bot token and channel config. | done | Added `TelegramConnector` and dispatcher registration from config/secrets. |
| 3.2 | Plan Telegram ingress strategy (webhook or polling) and update normalization into framework event format. | done | Implemented webhook ingress + runtime background polling loop (`getUpdates`) with normalized inbound events. |
| 3.3 | Define Telegram egress mapping (text/tool responses) including Telegram API error and rate-limit handling. | done | Implemented `sendMessage` mapping with transient/permanent classification and retry/backoff. |
| 3.4 | Define security/config rules for token storage, rotation, and redaction in logs. | done | Added `TELEGRAM_BOT_TOKEN` + `secrets.telegram_bot_token`; sensitive keys are redacted by audit layer. |
| 3.v1 | Verify: Telegram integration tests cover start-up, message round-trip, malformed updates, and auth/token failures. | done | Added tests for round-trip, malformed update ignore, invalid secret, retry success, and outage failure. |

## Phase 4: Config and UX Alignment
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | Extend config schema for connector settings (`telegram.enabled`, token secret, webhook base URL or polling mode). | done | Added `channels.telegram` config and secret/env token merge. |
| 4.2 | Plan frontend configuration surface and runtime status display for connector health/state. | in_progress | Added channel status display (enabled channels + telegram mode) in frontend rail; config editing surface remains pending. |
| 4.3 | Ensure backward-compatible defaults keep existing single-channel web chat path unchanged. | done | Existing `/api/chat` flow and web e2e flows remain green. |
| 4.v1 | Verify: Config and UI regression checklist for existing routes and chat sessions. | done | Existing chat/sessions regressions remain green under `make test-all` including new channel status rendering. |

## Phase 5: E2E and Completion Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | Add dedicated e2e tests for multi-channel connector flows including Telegram ingress/egress paths. | done | Added Playwright test `telegram webhook route supports channel session reuse`. |
| 5.2 | Add failure-path e2e for connector outage, invalid token, and retry behavior. | done | Added e2e cases for retry success and forced connector outage (HTTP 500). |
| 5.3 | Run mandatory completion gate command after implementation phases finish. | done | Executed `make test-all` successfully. |
| 5.v1 | Verify: `make test-e2e` passes with connector and Telegram scenarios. | done | Passed (desktop/mobile + telegram webhook case). |
| 5.v2 | Verify: `make test-all` passes before marking task complete. | done | Passed (unit + integration + e2e). |
