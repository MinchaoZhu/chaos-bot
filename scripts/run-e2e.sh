#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="${CHAOS_BOT_TMP_ROOT:-${ROOT_DIR}/.tmp/e2e}"
E2E_RUNTIME_DIR="${TMP_ROOT}/runtime"
E2E_ARTIFACTS_DIR="${TMP_ROOT}/artifacts"

pick_random_port() {
  node -e 'const net=require("node:net"); const server=net.createServer(); server.listen(0,"127.0.0.1",()=>{const address=server.address(); const port=typeof address === "object" && address ? address.port : 0; server.close(()=>process.stdout.write(String(port)));}); server.on("error",()=>process.exit(1));'
}

mkdir -p "${E2E_RUNTIME_DIR}" "${E2E_ARTIFACTS_DIR}"

E2E_PORT="${E2E_PORT:-$(pick_random_port)}"
E2E_BASE_URL="${E2E_BASE_URL:-http://127.0.0.1:${E2E_PORT}}"
E2E_SHELL_PORT="${E2E_SHELL_PORT:-$(pick_random_port)}"
while [ "${E2E_SHELL_PORT}" = "${E2E_PORT}" ]; do
  E2E_SHELL_PORT="$(pick_random_port)"
done
E2E_SHELL_BASE_URL="${E2E_SHELL_BASE_URL:-http://127.0.0.1:${E2E_SHELL_PORT}}"
E2E_REACT_RUNTIME_URL="${E2E_REACT_RUNTIME_URL:-${E2E_SHELL_BASE_URL}}"

export E2E_PORT
export E2E_BASE_URL
export E2E_SHELL_PORT
export E2E_SHELL_BASE_URL
export E2E_REACT_RUNTIME_URL
export E2E_TMP_DIR="${E2E_RUNTIME_DIR}"
export E2E_ARTIFACTS_DIR

echo "e2e backend: ${E2E_BASE_URL}"
echo "e2e shell: ${E2E_SHELL_BASE_URL}"

cd "${ROOT_DIR}/e2e"
test -d node_modules/@playwright/test || npm install
test -d "${ROOT_DIR}/frontend-react/node_modules/react" || npm --prefix "${ROOT_DIR}/frontend-react" install
npx playwright test
