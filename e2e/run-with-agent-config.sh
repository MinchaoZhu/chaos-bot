#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PORT="${E2E_PORT:-3010}"
E2E_TMP_DIR="${E2E_TMP_DIR:-${ROOT_DIR}/.tmp/e2e/runtime}"
RUNTIME_WORK_DIR="${E2E_TMP_DIR}/workspace"
TMP_AGENT_FILE="${RUNTIME_WORK_DIR}/agent.json"

mkdir -p "${E2E_TMP_DIR}" "${RUNTIME_WORK_DIR}"

cat >"${TMP_AGENT_FILE}" <<EOF
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
EOF

cleanup() {
  rm -f "${TMP_AGENT_FILE}"
}

trap cleanup EXIT

AGENT_CONFIG_PATH="${TMP_AGENT_FILE}" cargo run -p chaos-bot-backend
