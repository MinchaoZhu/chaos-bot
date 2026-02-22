#!/usr/bin/env bash
# Delete all runtime-generated files and restore a clean development state.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
rm -rf "$ROOT/memory" "$ROOT/data" "$ROOT/personality" "$ROOT/MEMORY.md"
echo "runtime files cleaned"
