#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SUITE_NAME="${1:?usage: run-test-suite.sh <suite-name> <command...>}"
shift

if [ "$#" -eq 0 ]; then
  echo "missing command to run for suite: ${SUITE_NAME}" >&2
  exit 1
fi

SUITE_TMP_DIR="${ROOT_DIR}/.tmp/${SUITE_NAME}"
rm -rf "${SUITE_TMP_DIR}"
mkdir -p "${SUITE_TMP_DIR}"

cleanup() {
  rm -rf "${SUITE_TMP_DIR}"
  rmdir "${ROOT_DIR}/.tmp" 2>/dev/null || true
}

trap cleanup EXIT

export TMPDIR="${SUITE_TMP_DIR}"
export TMP="${SUITE_TMP_DIR}"
export TEMP="${SUITE_TMP_DIR}"
export CHAOS_BOT_TMP_ROOT="${SUITE_TMP_DIR}"

"$@"
