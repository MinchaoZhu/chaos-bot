#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"
TMP_DIR="$(mktemp -d)"
PACKAGE_OUTPUTS="${TMP_DIR}/package-outputs.txt"
BACKEND_LOG="${TMP_DIR}/backend.log"
PORT=3311

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

bash "${ROOT_DIR}/scripts/release/package-linux-x86_64.sh" "${OUTPUT_DIR}" > "${PACKAGE_OUTPUTS}"

declare bundle_path=""
declare bundle_root=""
declare release_version=""
while IFS='=' read -r key value; do
  case "${key}" in
    bundle_path|bundle_root|release_version)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < "${PACKAGE_OUTPUTS}"

if [[ -z "${bundle_path}" || -z "${bundle_root}" || -z "${release_version}" ]]; then
  echo "failed to parse package outputs" >&2
  exit 1
fi

UNPACK_DIR="${TMP_DIR}/unpacked"
INSTALL_PREFIX="${TMP_DIR}/install-root"
HOME_DIR="${TMP_DIR}/home"
mkdir -p "${UNPACK_DIR}" "${INSTALL_PREFIX}" "${HOME_DIR}/.chaos-bot"
tar -C "${UNPACK_DIR}" -xzf "${bundle_path}"
"${UNPACK_DIR}/${bundle_root}/install.sh" --prefix "${INSTALL_PREFIX}"

cat > "${HOME_DIR}/.chaos-bot/config.json" <<EOF
{
  "server": {
    "host": "127.0.0.1",
    "port": ${PORT}
  },
  "llm": {
    "provider": "mock"
  }
}
EOF

HOME="${HOME_DIR}" CHAOS_BOT_DISABLE_SELF_RESTART=1 \
  "${INSTALL_PREFIX}/bin/chaos-bot" > "${BACKEND_LOG}" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 40); do
  if curl -fsS "http://127.0.0.1:${PORT}/api/health" >/dev/null; then
    break
  fi
  sleep 1
done

curl -fsS "http://127.0.0.1:${PORT}/api/health" >/dev/null
html="$(curl -fsS "http://127.0.0.1:${PORT}/")"
case "${html}" in
  *chaos-bot* ) ;;
  * )
    echo "packaged frontend root did not return the app shell" >&2
    exit 1
    ;;
esac

asset_file="$(find "${INSTALL_PREFIX}/share/chaos-bot/releases/${release_version}/frontend/assets" -type f | head -n 1)"
if [[ -z "${asset_file}" ]]; then
  echo "packaged frontend assets were not installed" >&2
  exit 1
fi

asset_name="$(basename "${asset_file}")"
asset_headers="$(curl -fsSI "http://127.0.0.1:${PORT}/assets/${asset_name}" | tr -d '\r' || true)"
if [[ "${asset_headers}" != *"200 OK"* ]]; then
  echo "packaged frontend asset did not respond successfully" >&2
  exit 1
fi

echo "packaged runtime verified for ${release_version}"
