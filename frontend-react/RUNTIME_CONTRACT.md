# Runtime Contract (Phase 1)

This contract keeps the new Tauri + React shell compatible with the existing backend HTTP/SSE API.

## Commands

| Command | Tauri invoke | Backend endpoint | Notes |
|---|---|---|---|
| `runtime.health` | `health` | `GET /api/health` | Health probe and time sync. |
| `session.list` | `list_sessions` | `GET /api/sessions` | Sorted by `updated_at` desc (backend behavior). |
| `session.create` | `create_session` | `POST /api/sessions` | Creates and returns empty session. |
| `session.get` | `get_session` | `GET /api/sessions/:id` | Fetches full message history. |
| `session.delete` | `delete_session` | `DELETE /api/sessions/:id` | Returns `204` on success. |
| `chat.stream` | `chat_stream` | `POST /api/chat` (SSE) | Emits stream events to frontend. |
| `config.get` | `get_config` | `GET /api/config` | Returns running/disk config state and raw json. |
| `config.apply` | `apply_config` | `POST /api/config/apply` | Apply raw/structured config and hot-rebuild runtime agent. |
| `config.reset` | `reset_config` | `POST /api/config/reset` | Restore disk config from running snapshot. |
| `config.restart` | `restart_config` | `POST /api/config/restart` | Request restart (or noop when restart disabled). |

## Stream Events

| Event | Payload shape |
|---|---|
| `session` | `{ "session_id": "..." }` |
| `delta` | `string` |
| `tool_call` | `{ id, name, args, output, is_error }` |
| `done` | `{ session_id, usage, finish_reason }` |
| `error` | `{ message }` |

Event channel for Tauri: `chaos://chat-event`

Envelope shape:

```json
{
  "stream_id": "<uuid>",
  "event": "delta",
  "data": "partial content"
}
```

## Error Codes

| Code | Meaning |
|---|---|
| `NETWORK_UNAVAILABLE` | Runtime cannot connect to backend. |
| `HTTP_BAD_REQUEST` | Invalid request payload (`400`). |
| `HTTP_UNAUTHORIZED` | Missing/invalid auth (`401/403`). |
| `HTTP_NOT_FOUND` | Endpoint or resource not found (`404`). |
| `HTTP_SERVER_ERROR` | Backend side failure (`5xx`). |
| `SSE_PROTOCOL_ERROR` | Invalid SSE frame or malformed event JSON. |
| `TAURI_INVOKE_FAILED` | Frontend `invoke` call failed. |
| `UNKNOWN` | Fallback for uncategorized errors. |

## Compatibility Notes

- Backend API remains unchanged in Phase 1.
- `frontend-react/` is the only maintained web frontend.
- Tauri runtime (`src-tauri/`) consumes the same HTTP/SSE contract.
