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

should_cleanup=1
cleanup() {
  if [ "${should_cleanup}" -eq 1 ]; then
    rm -rf "${SUITE_TMP_DIR}"
    rmdir "${ROOT_DIR}/.tmp" 2>/dev/null || true
  fi
}

trap cleanup EXIT

export TMPDIR="${SUITE_TMP_DIR}"
export TMP="${SUITE_TMP_DIR}"
export TEMP="${SUITE_TMP_DIR}"
export CHAOS_BOT_TMP_ROOT="${SUITE_TMP_DIR}"

set +e
"$@"
status=$?
set -e

if [ "${status}" -ne 0 ] && [ "${CHAOS_BOT_KEEP_TMP_ON_FAIL:-0}" = "1" ]; then
  should_cleanup=0
  echo "suite failed; preserving tmp artifacts at ${SUITE_TMP_DIR}" >&2
fi

exit "${status}"
