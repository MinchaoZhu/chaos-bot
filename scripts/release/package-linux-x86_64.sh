#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
OUTPUT_DIR="${1:-${ROOT_DIR}/.tmp/release}"
ARCHIVE_TARGET="linux-x86_64"
PACKAGE_NAME="chaos-bot"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT_DIR}/target}"

mkdir -p "${OUTPUT_DIR}"

declare base_version=""
declare release_version=""
declare tag_name=""
declare artifact_stem=""
declare git_short_sha=""
declare commit_count=""
declare github_repository=""
declare github_api_base_url=""
declare latest_release_url=""

while IFS='=' read -r key value; do
  case "${key}" in
    base_version|release_version|tag_name|artifact_stem|git_short_sha|commit_count)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/version.sh")

while IFS='=' read -r key value; do
  case "${key}" in
    github_repository|github_api_base_url|latest_release_url)
      printf -v "${key}" '%s' "${value}"
      ;;
  esac
done < <("${ROOT_DIR}/scripts/release/repository.sh")

bundle_root="${artifact_stem}-${ARCHIVE_TARGET}"
staging_dir="${OUTPUT_DIR}/${bundle_root}"
bundle_path="${OUTPUT_DIR}/${bundle_root}.tar.gz"
bundle_checksum_path="${bundle_path}.sha256"
bundle_manifest_path="${OUTPUT_DIR}/${bundle_root}.manifest.json"

rm -rf "${staging_dir}" "${bundle_path}" "${bundle_checksum_path}" "${bundle_manifest_path}"
mkdir -p "${staging_dir}/bin"

cargo build --manifest-path "${ROOT_DIR}/Cargo.toml" --target-dir "${CARGO_TARGET_DIR}" --release -p chaos-bot-backend \
  --bin chaos-bot >&2

cp "${CARGO_TARGET_DIR}/release/chaos-bot" "${staging_dir}/bin/chaos-bot"
cp "${ROOT_DIR}/VERSION" "${staging_dir}/VERSION"
cp "${ROOT_DIR}/README.md" "${staging_dir}/README.md"

cat > "${bundle_manifest_path}" <<EOF
{
  "project": "${PACKAGE_NAME}",
  "base_version": "${base_version}",
  "release_version": "${release_version}",
  "tag_name": "${tag_name}",
  "target": "${ARCHIVE_TARGET}",
  "repository": "${github_repository}",
  "release_api_base_url": "${github_api_base_url}",
  "latest_release_url": "${latest_release_url}",
  "archive_root": "${bundle_root}",
  "entrypoint_binary": "bin/chaos-bot",
  "installer": "install.sh",
  "install_layout": {
    "release_root": "share/chaos-bot/releases/${release_version}",
    "launchers": [
      "bin/chaos-bot"
    ]
  },
  "artifacts": [
    "${bundle_root}.tar.gz",
    "$(basename "${bundle_checksum_path}")",
    "$(basename "${bundle_manifest_path}")"
  ]
}
EOF

cp "${bundle_manifest_path}" "${staging_dir}/release-manifest.json"

cat > "${staging_dir}/install.sh" <<EOF
#!/usr/bin/env bash
set -euo pipefail

PACKAGE_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
PREFIX="\${HOME}/.local"

while [ "\$#" -gt 0 ]; do
  case "\$1" in
    --prefix)
      PREFIX="\$2"
      shift 2
      ;;
    *)
      echo "unknown argument: \$1" >&2
      exit 1
      ;;
  esac
done

RELEASE_VERSION="${release_version}"
RELEASE_ROOT="\${PREFIX}/share/chaos-bot/releases/\${RELEASE_VERSION}"
LAUNCHER_PATH="\${PREFIX}/bin/chaos-bot"
rm -rf "\${RELEASE_ROOT}"
mkdir -p "\${RELEASE_ROOT}" "\$(dirname "\${LAUNCHER_PATH}")"
cp -R "\${PACKAGE_DIR}/bin" "\${RELEASE_ROOT}/"
cp "\${PACKAGE_DIR}/VERSION" "\${RELEASE_ROOT}/VERSION"
cp "\${PACKAGE_DIR}/README.md" "\${RELEASE_ROOT}/README.md"
cp "\${PACKAGE_DIR}/release-manifest.json" "\${RELEASE_ROOT}/release-manifest.json"

cat > "\${LAUNCHER_PATH}" <<'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="\$(cd "\$(dirname "\${BASH_SOURCE[0]}")" && pwd)"
PREFIX="\$(cd "\${SCRIPT_DIR}/.." && pwd)"
RELEASE_VERSION="${release_version}"
RELEASE_ROOT="\${PREFIX}/share/chaos-bot/releases/\${RELEASE_VERSION}"
export CHAOS_BOT_INSTALL_PREFIX="\${PREFIX}"
export CHAOS_BOT_RELEASE_VERSION="\${RELEASE_VERSION}"
export CHAOS_BOT_RELEASE_ROOT="\${RELEASE_ROOT}"
export CHAOS_BOT_UPGRADE_REPOSITORY="${github_repository}"
export CHAOS_BOT_UPGRADE_API_BASE_URL="${github_api_base_url}"
exec "\${RELEASE_ROOT}/bin/chaos-bot" "\$@"
LAUNCHER

chmod +x \
  "\${LAUNCHER_PATH}" \
  "\${RELEASE_ROOT}/bin/chaos-bot"
echo "installed chaos-bot ${release_version} to \${RELEASE_ROOT}"
echo "launcher available at \${LAUNCHER_PATH}"
EOF

chmod +x "${staging_dir}/install.sh" "${staging_dir}/bin/chaos-bot"

tar -C "${OUTPUT_DIR}" -czf "${bundle_path}" "${bundle_root}"
(cd "${OUTPUT_DIR}" && sha256sum "$(basename "${bundle_path}")" > "$(basename "${bundle_checksum_path}")")

echo "bundle_path=${bundle_path}"
echo "bundle_checksum_path=${bundle_checksum_path}"
echo "bundle_manifest_path=${bundle_manifest_path}"
echo "bundle_name=$(basename "${bundle_path}")"
echo "bundle_root=${bundle_root}"
echo "release_version=${release_version}"
