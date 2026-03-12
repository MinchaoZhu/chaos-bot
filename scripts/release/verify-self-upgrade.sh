#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"
TMP_DIR="$(mktemp -d)"
PACKAGE_OUTPUTS="${TMP_DIR}/package-outputs.txt"
ASSET_PORT=3391

cleanup() {
  if [[ -n "${HTTP_PID:-}" ]]; then
    kill "${HTTP_PID}" >/dev/null 2>&1 || true
    wait "${HTTP_PID}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${RELAUNCH_PID:-}" ]]; then
    kill "${RELAUNCH_PID}" >/dev/null 2>&1 || true
    wait "${RELAUNCH_PID}" >/dev/null 2>&1 || true
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

if [[ ! "${release_version}" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  echo "expected semver release version, got ${release_version}" >&2
  exit 1
fi
next_release_version="${BASH_REMATCH[1]}.${BASH_REMATCH[2]}.$((BASH_REMATCH[3] + 1))"
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
  "logging": {
    "level": "error"
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

status_json="$(
  HOME="${HOME_DIR}" \
  CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
    "${INSTALL_PREFIX}/bin/chaos-bot" --output json upgrade status
)"
python3 - "${next_release_version}" "${status_json}" <<'PY'
import json
import sys

status = json.loads(sys.argv[2])
expected = sys.argv[1]
assert status["supported"] is True
assert status["upgrade_available"] is True
assert status["latest_version"] == expected
PY

apply_json="$(
  HOME="${HOME_DIR}" \
  CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
    "${INSTALL_PREFIX}/bin/chaos-bot" --output json upgrade apply
)"
python3 - "${next_release_version}" "${apply_json}" <<'PY'
import json
import sys

result = json.loads(sys.argv[2])
expected = sys.argv[1]
assert result["action"] == "upgrade"
assert result["target_version"] == expected
assert result["relaunch_required"] is True
PY

test -d "${INSTALL_PREFIX}/share/chaos-bot/releases/${next_release_version}"
grep -q "${next_release_version}" "${INSTALL_PREFIX}/bin/chaos-bot"

relaunch_json="$(
HOME="${HOME_DIR}" \
  CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
  "${INSTALL_PREFIX}/bin/chaos-bot" --output json upgrade relaunch
)"
python3 - "${next_release_version}" "${relaunch_json}" <<'PY'
import json
import sys

result = json.loads(sys.argv[2])
expected = sys.argv[1]
assert result["action"] == "relaunch"
assert result["target_version"] == expected
PY

post_upgrade_status="$(
  HOME="${HOME_DIR}" \
  CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL="http://127.0.0.1:${ASSET_PORT}/repos/test/chaos-bot/releases/latest" \
    "${INSTALL_PREFIX}/bin/chaos-bot" --output json upgrade status
)"
python3 - "${next_release_version}" "${post_upgrade_status}" <<'PY'
import json
import sys

status = json.loads(sys.argv[2])
expected = sys.argv[1]
assert status["current_version"] == expected
assert status["upgrade_available"] is False
PY

echo "self-upgrade verified from ${release_version} to ${next_release_version}"
