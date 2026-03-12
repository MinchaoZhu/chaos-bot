#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"
TMP_DIR="$(mktemp -d)"
PACKAGE_OUTPUTS="${TMP_DIR}/package-outputs.txt"
ASSET_PORT=3392

cleanup() {
  if [[ -n "${HTTP_PID:-}" ]]; then
    kill "${HTTP_PID}" >/dev/null 2>&1 || true
    wait "${HTTP_PID}" >/dev/null 2>&1 || true
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

ASSET_ROOT="${TMP_DIR}/release-server"
INSTALL_PREFIX="${TMP_DIR}/install-root"
HOME_DIR="${TMP_DIR}/home"
mkdir -p "${ASSET_ROOT}/assets" "${ASSET_ROOT}/repos/test/chaos-bot/releases" "${INSTALL_PREFIX}" "${HOME_DIR}/.chaos-bot"

cp "${bundle_path}" "${ASSET_ROOT}/assets/${bundle_root}.tar.gz"

cat > "${ASSET_ROOT}/repos/test/chaos-bot/releases/latest" <<EOF
{
  "tag_name": "v${release_version}",
  "assets": [
    { "name": "${bundle_root}.tar.gz", "browser_download_url": "http://127.0.0.1:${ASSET_PORT}/assets/${bundle_root}.tar.gz" }
  ]
}
EOF

(
  cd "${ASSET_ROOT}"
  python3 -m http.server "${ASSET_PORT}" --bind 127.0.0.1 >/tmp/chaos-bot-installer-http.log 2>&1
) &
HTTP_PID=$!

for _ in $(seq 1 20); do
  if curl -fsS "http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" >/dev/null; then
    break
  fi
  sleep 1
done

curl -fsS "http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" >/dev/null

CHAOS_BOT_INSTALL_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
  "${ROOT_DIR}/scripts/install-from-github.sh" --prefix "${INSTALL_PREFIX}"

cat > "${HOME_DIR}/.chaos-bot/config.json" <<EOF
{
  "logging": {
    "level": "error"
  },
  "llm": {
    "provider": "mock"
  }
}
EOF

test -x "${INSTALL_PREFIX}/bin/chaos-bot"
test -d "${INSTALL_PREFIX}/share/chaos-bot/releases/${release_version}"
HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --help >/dev/null
HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" chat "installer smoke" >/dev/null

echo "github installer verified for ${release_version}"
