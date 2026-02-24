#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="${CHAOS_BOT_TMP_ROOT:-${ROOT_DIR}/.tmp/e2e}"
E2E_RUNTIME_DIR="${TMP_ROOT}/runtime"
E2E_ARTIFACTS_DIR="${TMP_ROOT}/artifacts"

mkdir -p "${E2E_RUNTIME_DIR}" "${E2E_ARTIFACTS_DIR}"

export E2E_TMP_DIR="${E2E_RUNTIME_DIR}"
export E2E_ARTIFACTS_DIR

cd "${ROOT_DIR}/e2e"
test -d node_modules/@playwright/test || npm install
npx playwright test
