#!/usr/bin/env bash
# Delete all runtime-generated files and restore a clean development state.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOME_DIR="${HOME:-$ROOT}"
WORKSPACE="${CHAOS_BOT_WORKSPACE:-$HOME_DIR/.chaos-bot}"

if [[ "$WORKSPACE" == "/" ]]; then
  echo "refusing to clean workspace: /"
  exit 1
fi

rm -rf "$WORKSPACE" "$ROOT/.tmp"
echo "runtime files cleaned: workspace=$WORKSPACE tmp=$ROOT/.tmp"
