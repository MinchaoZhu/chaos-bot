#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_PATH="${1:-${ROOT_DIR}/.tmp/release/release-notes.md}"

mkdir -p "$(dirname "${OUTPUT_PATH}")"

declare base_version=""
declare release_version=""
declare tag_name=""
declare artifact_stem=""
declare git_short_sha=""
declare commit_count=""
declare github_repository=""

while IFS='=' read -r key value; do
  case "${key}" in
    base_version|release_version|tag_name|artifact_stem|git_short_sha|commit_count)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/version.sh")

while IFS='=' read -r key value; do
  case "${key}" in
    github_repository)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/repository.sh")

previous_tag="$(git -C "${ROOT_DIR}" describe --tags --abbrev=0 --match 'v*' HEAD^ 2>/dev/null || true)"
commit_range="HEAD"
range_label="repository start..HEAD"
history_url=""
if [[ -n "${previous_tag}" ]]; then
  commit_range="${previous_tag}..HEAD"
  range_label="${previous_tag}..HEAD"
fi

release_kind="Incremental release"
if [[ -z "${previous_tag}" ]]; then
  release_kind="Initial release"
fi

commit_total="$(git -C "${ROOT_DIR}" rev-list --count "${commit_range}")"

declare -a highlights=()
declare -a fixes=()
declare -a docs=()
declare -a maintenance=()
declare -a other=()

while IFS=$'\t' read -r short_sha subject; do
  [[ -n "${short_sha}" ]] || continue

  line="- ${subject} (\`${short_sha}\`)"
  normalized_subject="${subject,,}"

  case "${normalized_subject}" in
    feat*|feature*|add*|implement*|support*|introduce* )
      highlights+=("${line}")
      ;;
    fix*|bugfix*|correct*|resolve* )
      fixes+=("${line}")
      ;;
    docs*|doc*|readme* )
      docs+=("${line}")
      ;;
    chore*|refactor*|perf*|test*|build*|ci*|style* )
      maintenance+=("${line}")
      ;;
    merge* )
      ;;
    * )
      other+=("${line}")
      ;;
  esac
done < <(git -C "${ROOT_DIR}" log --no-merges --reverse --format=$'%h\t%s' "${commit_range}")

if [[ -n "${github_repository}" ]]; then
  if [[ -n "${previous_tag}" ]]; then
    history_url="https://github.com/${github_repository}/compare/${previous_tag}...${git_short_sha}"
  else
    history_url="https://github.com/${github_repository}/commits/${git_short_sha}"
  fi
fi

{
  echo "# chaos-bot ${release_version}"
  echo
  echo "- Release type: ${release_kind}"
  echo "- Base version: \`${base_version}\`"
  echo "- Release tag: \`${tag_name}\`"
  echo "- Commit: \`${git_short_sha}\`"
  echo "- Commit count: \`${commit_count}\`"
  echo "- Changes since: \`${range_label}\`"
  echo "- Included commits: \`${commit_total}\`"
  echo "- Publish branch: \`master\`"
  echo "- Install bundle: \`${artifact_stem}-linux-x86_64.tar.gz\`"

  if [[ ${#highlights[@]} -gt 0 ]]; then
    echo
    echo "## Highlights"
    printf '%s\n' "${highlights[@]}"
  fi

  if [[ ${#fixes[@]} -gt 0 ]]; then
    echo
    echo "## Fixes"
    printf '%s\n' "${fixes[@]}"
  fi

  if [[ ${#docs[@]} -gt 0 ]]; then
    echo
    echo "## Docs"
    printf '%s\n' "${docs[@]}"
  fi

  if [[ ${#maintenance[@]} -gt 0 ]]; then
    echo
    echo "## Maintenance"
    printf '%s\n' "${maintenance[@]}"
  fi

  if [[ ${#other[@]} -gt 0 ]]; then
    echo
    echo "## Other changes"
    printf '%s\n' "${other[@]}"
  fi

  if [[ ${#highlights[@]} -eq 0 && ${#fixes[@]} -eq 0 && ${#docs[@]} -eq 0 && ${#maintenance[@]} -eq 0 && ${#other[@]} -eq 0 ]]; then
    echo
    echo "## Changes"
    echo "- No non-merge commits were found in \`${range_label}\`."
  fi

  if [[ -n "${history_url}" ]]; then
    echo
    echo "Full commit history: ${history_url}"
  fi
} > "${OUTPUT_PATH}"

echo "notes_path=${OUTPUT_PATH}"
