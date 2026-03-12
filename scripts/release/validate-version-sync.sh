#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION_FILE="${ROOT_DIR}/VERSION"
require_change_ref=""

usage() {
  echo "usage: $0 [--require-change <git-ref>]" >&2
}

while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --require-change)
      if [[ "$#" -lt 2 ]]; then
        usage
        exit 1
      fi
      require_change_ref="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      usage
      exit 1
      ;;
  esac
done

trimmed_file_value() {
  tr -d '[:space:]' < "$1"
}

extract_toml_version() {
  sed -nE 's/^version = "([^"]+)"$/\1/p' "$1" | head -n 1
}

extract_package_json_version() {
  sed -nE 's/^[[:space:]]*"version":[[:space:]]*"([^"]+)",?$/\1/p' "$1" | head -n 1
}

expected_version="$(trimmed_file_value "${VERSION_FILE}")"

if [[ -z "${expected_version}" ]]; then
  echo "VERSION file is empty" >&2
  exit 1
fi

declare -A actual_versions=(
  ["backend/Cargo.toml"]="$(extract_toml_version "${ROOT_DIR}/backend/Cargo.toml")"
)

failed=0
for path in "${!actual_versions[@]}"; do
  actual="${actual_versions[$path]}"
  if [[ "${actual}" != "${expected_version}" ]]; then
    echo "version mismatch: ${path} has '${actual}', expected '${expected_version}'" >&2
    failed=1
  fi
done

if [[ "${failed}" -ne 0 ]]; then
  exit 1
fi

if [[ -n "${require_change_ref}" ]]; then
  if [[ "${require_change_ref}" =~ ^0+$ ]]; then
    echo "skip version change check for initial push"
  else
    previous_version="$(git -C "${ROOT_DIR}" show "${require_change_ref}:VERSION" 2>/dev/null | tr -d '[:space:]' || true)"
    if [[ -z "${previous_version}" ]]; then
      echo "failed to read VERSION from ${require_change_ref}" >&2
      exit 1
    fi
    if [[ "${previous_version}" == "${expected_version}" ]]; then
      echo "VERSION must be updated for this push: still '${expected_version}' since ${require_change_ref}" >&2
      exit 1
    fi
    echo "version change check passed: ${previous_version} -> ${expected_version}"
  fi
fi

echo "version sync check passed for ${expected_version}"
