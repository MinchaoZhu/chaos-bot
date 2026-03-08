#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"
TMP_DIR="$(mktemp -d)"
PACKAGE_OUTPUTS="${TMP_DIR}/package-outputs.txt"
CURRENT_PORT=3312
ASSET_PORT=3391

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "${SERVER_PID}" >/dev/null 2>&1 || true
    wait "${SERVER_PID}" >/dev/null 2>&1 || true
  fi
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

next_release_version="$(printf '%s' "${release_version}" | sed 's/\.[0-9][0-9]*$/.99999/')"
if [[ "${next_release_version}" = "${release_version}" ]]; then
  next_release_version="${release_version}.99999"
fi
next_artifact_stem="${next_release_version}"
next_bundle_root="${next_artifact_stem}-linux-x86_64"

UNPACK_DIR="${TMP_DIR}/unpacked"
INSTALL_PREFIX="${TMP_DIR}/install-root"
HOME_DIR="${TMP_DIR}/home"
ASSET_ROOT="${TMP_DIR}/release-server"
mkdir -p "${UNPACK_DIR}" "${INSTALL_PREFIX}" "${HOME_DIR}/.chaos-bot" "${ASSET_ROOT}/assets" "${ASSET_ROOT}/repos/test/chaos-bot/releases"

tar -C "${UNPACK_DIR}" -xzf "${bundle_path}"
"${UNPACK_DIR}/${bundle_root}/install.sh" --prefix "${INSTALL_PREFIX}"

cat > "${HOME_DIR}/.chaos-bot/config.json" <<EOF
{
  "server": {
    "host": "127.0.0.1",
    "port": ${CURRENT_PORT}
  },
  "llm": {
    "provider": "mock"
  }
}
EOF

STAGED_DIR="${TMP_DIR}/${next_bundle_root}"
cp -R "${UNPACK_DIR}/${bundle_root}" "${STAGED_DIR}"
sed -i "s/${release_version}/${next_release_version}/g" "${STAGED_DIR}/install.sh" "${STAGED_DIR}/release-manifest.json"

NEXT_BUNDLE_PATH="${ASSET_ROOT}/assets/${next_bundle_root}.tar.gz"
tar -C "${TMP_DIR}" -czf "${NEXT_BUNDLE_PATH}" "${next_bundle_root}"
(cd "${ASSET_ROOT}/assets" && sha256sum "$(basename "${NEXT_BUNDLE_PATH}")" > "$(basename "${NEXT_BUNDLE_PATH}").sha256")

cat > "${ASSET_ROOT}/assets/${next_bundle_root}.manifest.json" <<EOF
{
  "release_version": "${next_release_version}"
}
EOF

cat > "${ASSET_ROOT}/assets/release-metadata.json" <<EOF
{
  "project": "chaos-bot",
  "base_version": "$(tr -d '[:space:]' < "${ROOT_DIR}/VERSION")",
  "release_version": "${next_release_version}",
  "tag_name": "v${next_release_version}",
  "artifact_stem": "${next_artifact_stem}"
}
EOF
(cd "${ASSET_ROOT}/assets" && sha256sum release-metadata.json > release-metadata.sha256)

cat > "${ASSET_ROOT}/repos/test/chaos-bot/releases/latest" <<EOF
{
  "tag_name": "v${next_release_version}",
  "assets": [
    { "name": "release-metadata.json", "browser_download_url": "http://127.0.0.1:${ASSET_PORT}/assets/release-metadata.json" },
    { "name": "release-metadata.sha256", "browser_download_url": "http://127.0.0.1:${ASSET_PORT}/assets/release-metadata.sha256" },
    { "name": "${next_bundle_root}.tar.gz", "browser_download_url": "http://127.0.0.1:${ASSET_PORT}/assets/${next_bundle_root}.tar.gz" },
    { "name": "${next_bundle_root}.tar.gz.sha256", "browser_download_url": "http://127.0.0.1:${ASSET_PORT}/assets/${next_bundle_root}.tar.gz.sha256" },
    { "name": "${next_bundle_root}.manifest.json", "browser_download_url": "http://127.0.0.1:${ASSET_PORT}/assets/${next_bundle_root}.manifest.json" }
  ]
}
EOF

(
  cd "${ASSET_ROOT}"
  python3 -m http.server "${ASSET_PORT}" --bind 127.0.0.1 >/tmp/chaos-bot-upgrade-http.log 2>&1
) &
HTTP_PID=$!

for _ in $(seq 1 20); do
  if curl -fsS "http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" >/dev/null; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" >/dev/null

HOME="${HOME_DIR}" CHAOS_BOT_DISABLE_SELF_RESTART=1 \
  CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
  "${INSTALL_PREFIX}/bin/chaos-bot" >/tmp/chaos-bot-upgrade-backend.log 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 40); do
  if curl -fsS "http://127.0.0.1:${CURRENT_PORT}/api/health" >/dev/null; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:${CURRENT_PORT}/api/health" >/dev/null

status_json="$(curl -fsS "http://127.0.0.1:${CURRENT_PORT}/api/upgrade")"
case "${status_json}" in
  *"\"upgrade_available\":true"* ) ;;
  * )
    echo "upgrade status did not report an available release" >&2
    exit 1
    ;;
esac

apply_json="$(curl -fsS -H 'content-type: application/json' -d '{}' "http://127.0.0.1:${CURRENT_PORT}/api/upgrade/apply")"
case "${apply_json}" in
  *"\"action\":\"upgrade\""* ) ;;
  * )
    echo "upgrade apply did not return an upgrade action" >&2
    exit 1
    ;;
esac
case "${apply_json}" in
  *"\"target_version\":\"${next_release_version}\""* ) ;;
  * )
    echo "upgrade apply did not target the staged release version" >&2
    exit 1
    ;;
esac

test -d "${INSTALL_PREFIX}/share/chaos-bot/releases/${next_release_version}"
grep -q "${next_release_version}" "${INSTALL_PREFIX}/bin/chaos-bot"

kill "${SERVER_PID}" >/dev/null 2>&1 || true
wait "${SERVER_PID}" >/dev/null 2>&1 || true
unset SERVER_PID

HOME="${HOME_DIR}" CHAOS_BOT_DISABLE_SELF_RESTART=1 \
  CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
  "${INSTALL_PREFIX}/bin/chaos-bot" >/tmp/chaos-bot-upgrade-backend-relaunch.log 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 40); do
  if curl -fsS "http://127.0.0.1:${CURRENT_PORT}/api/health" >/dev/null; then
    break
  fi
  sleep 1
done
curl -fsS "http://127.0.0.1:${CURRENT_PORT}/api/health" >/dev/null

relaunch_status_json="$(curl -fsS "http://127.0.0.1:${CURRENT_PORT}/api/upgrade")"
case "${relaunch_status_json}" in
  *"\"current_version\":\"${next_release_version}\""* ) ;;
  * )
    echo "relaunched backend did not report the upgraded current version" >&2
    exit 1
    ;;
esac
case "${relaunch_status_json}" in
  *"\"upgrade_available\":false"* ) ;;
  * )
    echo "relaunched backend still reports an available upgrade" >&2
    exit 1
    ;;
esac

echo "self-upgrade verified from ${release_version} to ${next_release_version}"
