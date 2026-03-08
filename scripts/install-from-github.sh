#!/usr/bin/env bash
set -euo pipefail

GITHUB_REPOSITORY="${GITHUB_REPOSITORY:-MinchaoZhu/chaos-bot}"
GITHUB_API_URL="${GITHUB_API_URL:-https://api.github.com}"
CHAOS_BOT_INSTALL_PREFIX="${CHAOS_BOT_INSTALL_PREFIX:-${HOME}/.local}"
WORK_DIR=""

usage() {
  cat >&2 <<'EOF'
usage: install-from-github.sh [--prefix <path>]

Downloads the latest Linux x86_64 release bundle from GitHub Releases and installs it.
EOF
}

cleanup() {
  if [[ -n "${WORK_DIR}" && -d "${WORK_DIR}" ]]; then
    rm -rf "${WORK_DIR}"
  fi
}
trap cleanup EXIT

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --prefix)
      if [[ "$#" -lt 2 ]]; then
        usage
        exit 1
      fi
      CHAOS_BOT_INSTALL_PREFIX="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_cmd curl
require_cmd tar
require_cmd python3
require_cmd bash

latest_release_url="${CHAOS_BOT_INSTALL_LATEST_RELEASE_URL:-${GITHUB_API_URL%/}/repos/${GITHUB_REPOSITORY}/releases/latest}"
WORK_DIR="$(mktemp -d)"
release_json="${WORK_DIR}/latest-release.json"

echo "fetching latest release metadata from ${latest_release_url}" >&2
curl -fsSL "${latest_release_url}" -o "${release_json}"

read_release_field() {
  local field="$1"
  python3 - "$release_json" "$field" <<'PY'
import json
import sys

path, field = sys.argv[1], sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)

value = data.get(field, "")
if isinstance(value, str):
    print(value)
PY
}

find_bundle_url() {
  python3 - "$release_json" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)

for asset in data.get("assets", []):
    name = asset.get("name", "")
    if name.endswith("-linux-x86_64.tar.gz"):
        print(asset.get("browser_download_url", ""))
        break
PY
}

tag_name="$(read_release_field tag_name)"
bundle_url="$(find_bundle_url)"

if [[ -z "${tag_name}" ]]; then
  echo "latest release is missing tag_name" >&2
  exit 1
fi

if [[ -z "${bundle_url}" ]]; then
  echo "latest release does not contain a linux-x86_64 bundle" >&2
  exit 1
fi

bundle_name="$(basename "${bundle_url}")"
bundle_path="${WORK_DIR}/${bundle_name}"
unpack_dir="${WORK_DIR}/bundle"

echo "downloading ${bundle_name}" >&2
curl -fsSL "${bundle_url}" -o "${bundle_path}"

mkdir -p "${unpack_dir}"
tar -xzf "${bundle_path}" -C "${unpack_dir}"

bundle_root="$(find "${unpack_dir}" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
if [[ -z "${bundle_root}" || ! -f "${bundle_root}/install.sh" ]]; then
  echo "downloaded bundle is missing install.sh" >&2
  exit 1
fi

echo "installing ${tag_name} into ${CHAOS_BOT_INSTALL_PREFIX}" >&2
bash "${bundle_root}/install.sh" --prefix "${CHAOS_BOT_INSTALL_PREFIX}"

echo "installation complete"
echo "launcher: ${CHAOS_BOT_INSTALL_PREFIX}/bin/chaos-bot"
