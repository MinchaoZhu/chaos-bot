#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"

mkdir -p "${OUTPUT_DIR}"

declare base_version=""
declare release_version=""
declare tag_name=""
declare artifact_stem=""
declare git_short_sha=""
declare commit_count=""
declare github_repository=""
declare latest_release_url=""
declare notes_path=""

while IFS='=' read -r key value; do
  case "${key}" in
    base_version|release_version|tag_name|artifact_stem|git_short_sha|commit_count)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/version.sh")

while IFS='=' read -r key value; do
  case "${key}" in
    github_repository|latest_release_url)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/repository.sh")

metadata_path="${OUTPUT_DIR}/release-metadata.json"
checksum_path="${OUTPUT_DIR}/release-metadata.sha256"

while IFS='=' read -r key value; do
  case "${key}" in
    notes_path)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/generate-release-notes.sh" "${OUTPUT_DIR}/release-notes.md")

cat > "${metadata_path}" <<EOF
{
  "project": "chaos-bot",
  "base_version": "${base_version}",
  "release_version": "${release_version}",
  "tag_name": "${tag_name}",
  "artifact_stem": "${artifact_stem}",
  "git_short_sha": "${git_short_sha}",
  "commit_count": ${commit_count},
  "published_from_branch": "master",
  "repository": "${github_repository}",
  "latest_release_url": "${latest_release_url}",
  "artifacts": [
    "release-metadata.json",
    "release-metadata.sha256",
    "${artifact_stem}-linux-x86_64.tar.gz",
    "${artifact_stem}-linux-x86_64.tar.gz.sha256",
    "${artifact_stem}-linux-x86_64.manifest.json"
  ]
}
EOF

(cd "${OUTPUT_DIR}" && sha256sum "$(basename "${metadata_path}")" > "$(basename "${checksum_path}")")

echo "metadata_path=${metadata_path}"
echo "notes_path=${notes_path}"
echo "checksum_path=${checksum_path}"
