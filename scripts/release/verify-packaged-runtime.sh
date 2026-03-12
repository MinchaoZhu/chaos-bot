#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"
TMP_DIR="$(mktemp -d)"
PACKAGE_OUTPUTS="${TMP_DIR}/package-outputs.txt"

cleanup() {
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
  "logging": {
    "level": "error"
  },
  "llm": {
    "provider": "mock"
  }
}
EOF

test -x "${INSTALL_PREFIX}/bin/chaos-bot"
test -x "${INSTALL_PREFIX}/share/chaos-bot/releases/${release_version}/bin/chaos-bot"
test ! -e "${INSTALL_PREFIX}/share/chaos-bot/releases/${release_version}/frontend"

python3 - "${INSTALL_PREFIX}/share/chaos-bot/releases/${release_version}/release-manifest.json" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    manifest = json.load(fh)

assert manifest["entrypoint_binary"] == "bin/chaos-bot"
assert manifest["install_layout"]["launchers"] == ["bin/chaos-bot"]
assert "frontend_dist" not in manifest
PY

help_output="$(HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --help)"
case "${help_output}" in
  *"CLI-first runtime for chaos-bot"* ) ;;
  * )
    echo "packaged launcher help output is missing CLI description" >&2
    exit 1
    ;;
esac

chat_json="$(HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --output json chat "packaged runtime hello")"
session_id="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["session_id"])' <<< "${chat_json}")"
if [[ -z "${session_id}" ]]; then
  echo "packaged chat did not return a session id" >&2
  exit 1
fi

sessions_json="$(HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --output json sessions list)"
python3 - "${session_id}" "${sessions_json}" <<'PY'
import json
import sys

session_id = sys.argv[1]
items = json.loads(sys.argv[2])
assert any(item["id"] == session_id for item in items)
PY

stream_output="$(HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --output jsonl chat --stream "packaged runtime stream")"
case "${stream_output}" in
  *'"event":"session"'*'"event":"done"'* | *'"event":"done"'*'"event":"session"'* ) ;;
  * )
    echo "packaged chat stream did not emit session/done events" >&2
    exit 1
    ;;
esac

continued_json="$(
  HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --output json chat "${session_id}" "second turn"
)"
python3 - "${session_id}" "${continued_json}" <<'PY'
import json
import sys

session_id = sys.argv[1]
result = json.loads(sys.argv[2])
assert result["session_id"] == session_id
PY

HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --output json \
  config apply --raw '{"llm":{"provider":"mock","model":"packaged-model"},"logging":{"level":"error"}}' >/dev/null
config_json="$(HOME="${HOME_DIR}" "${INSTALL_PREFIX}/bin/chaos-bot" --output json config get)"
config_model="$(python3 -c 'import json,sys; print(json.load(sys.stdin)["running"]["llm"]["model"])' <<< "${config_json}")"
if [[ "${config_model}" != "packaged-model" ]]; then
  echo "packaged config mutation did not persist expected model" >&2
  exit 1
fi

echo "packaged runtime verified for ${release_version}"
