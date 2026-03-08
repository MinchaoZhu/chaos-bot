#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION_FILE="${ROOT_DIR}/VERSION"

if [[ ! -f "${VERSION_FILE}" ]]; then
  echo "VERSION file is missing at ${VERSION_FILE}" >&2
  exit 1
fi

base_version="$(tr -d '[:space:]' < "${VERSION_FILE}")"
if [[ -z "${base_version}" ]]; then
  echo "VERSION file is empty" >&2
  exit 1
fi

git_short_sha="$(git -C "${ROOT_DIR}" rev-parse --short HEAD)"
commit_count="$(git -C "${ROOT_DIR}" rev-list --count HEAD)"
release_version="${base_version}"
tag_name="v${release_version}"
artifact_stem="${release_version}"

for kv in \
  "base_version=${base_version}" \
  "release_version=${release_version}" \
  "tag_name=${tag_name}" \
  "artifact_stem=${artifact_stem}" \
  "git_short_sha=${git_short_sha}" \
  "commit_count=${commit_count}"
do
  echo "${kv}"
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "${kv}" >> "${GITHUB_OUTPUT}"
  fi
done
