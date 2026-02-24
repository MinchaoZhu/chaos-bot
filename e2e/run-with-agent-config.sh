#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${E2E_PORT:-3010}"
E2E_TMP_DIR="${E2E_TMP_DIR:-${ROOT_DIR}/.tmp/e2e/runtime}"
E2E_HOME_DIR="${E2E_TMP_DIR}/home"
ORIGINAL_HOME="${HOME:-}"
CONFIG_DIR="${E2E_HOME_DIR}/.chaos-bot"
CONFIG_FILE="${CONFIG_DIR}/config.json"
RUNTIME_WORK_DIR="${E2E_TMP_DIR}/workspace"

mkdir -p "${E2E_TMP_DIR}" "${E2E_HOME_DIR}" "${CONFIG_DIR}" "${RUNTIME_WORK_DIR}"

cat >"${CONFIG_FILE}" <<EOF_CONFIG
{
  "server": {
    "host": "127.0.0.1",
    "port": ${PORT}
  },
  "llm": {
    "provider": "mock",
    "model": "mock",
    "temperature": 0.2,
    "max_tokens": 1024,
    "max_iterations": 6,
    "token_budget": 12000
  },
  "workspace": "${RUNTIME_WORK_DIR}",
  "logging": {
    "level": "info",
    "retention_days": 7,
    "directory": "logs"
  },
  "secrets": {}
}
EOF_CONFIG

HOME="${E2E_HOME_DIR}" \
RUSTUP_HOME="${RUSTUP_HOME:-${ORIGINAL_HOME}/.rustup}" \
CARGO_HOME="${CARGO_HOME:-${ORIGINAL_HOME}/.cargo}" \
CHAOS_BOT_DISABLE_SELF_RESTART=1 \
cargo run -p chaos-bot-backend
