#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION_FILE="${ROOT_DIR}/VERSION"

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
  ["frontend-react/package.json"]="$(extract_package_json_version "${ROOT_DIR}/frontend-react/package.json")"
  ["src-tauri/Cargo.toml"]="$(extract_toml_version "${ROOT_DIR}/src-tauri/Cargo.toml")"
  ["src-tauri/tauri.conf.json"]="$(extract_package_json_version "${ROOT_DIR}/src-tauri/tauri.conf.json")"
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

echo "version sync check passed for ${expected_version}"
