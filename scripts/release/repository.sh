#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
remote_url="${GITHUB_SERVER_URL:-https://github.com}"
api_base_url="${GITHUB_API_URL:-https://api.github.com}"
repository="${GITHUB_REPOSITORY:-}"

if [[ -z "${repository}" ]]; then
  origin="$(git -C "${ROOT_DIR}" remote get-url origin 2>/dev/null || true)"
  case "${origin}" in
    git@github.com:*)
      repository="${origin#git@github.com:}"
      repository="${repository%.git}"
      ;;
    https://github.com/*)
      repository="${origin#https://github.com/}"
      repository="${repository%.git}"
      ;;
  esac
fi

echo "github_server_url=${remote_url}"
echo "github_api_base_url=${api_base_url}"
echo "github_repository=${repository}"
if [[ -n "${repository}" ]]; then
  echo "latest_release_url=${api_base_url%/}/repos/${repository}/releases/latest"
fi
