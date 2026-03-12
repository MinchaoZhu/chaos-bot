use crate::domain::ports::UpgradePort;
use crate::domain::upgrade::{UpgradeApplyResult, UpgradeRestartResult, UpgradeStatus};
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use flate2::read::GzDecoder;
use reqwest::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tar::Archive;
use tempfile::TempDir;
use tokio::process::Command;

const DEFAULT_GITHUB_API_BASE_URL: &str = "https://api.github.com";
const DEFAULT_BUNDLE_TARGET: &str = "linux-x86_64";

#[derive(Clone)]
pub struct GitHubReleaseUpdater {
    client: Client,
}

impl GitHubReleaseUpdater {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("chaos-bot-updater")
            .build()
            .context("failed to build upgrade HTTP client")?;
        Ok(Self { client })
    }
}

#[async_trait]
impl UpgradePort for GitHubReleaseUpdater {
    async fn status(&self) -> Result<UpgradeStatus> {
        let Some(context) = load_install_context()? else {
            return Ok(unsupported_status(
                "self-upgrade is only available for installed release bundles",
            ));
        };

        let release = match resolve_release(&self.client, &context).await {
            Ok(release) => release,
            Err(error) => {
                return Ok(UpgradeStatus {
                    supported: true,
                    current_version: Some(context.current_version.clone()),
                    latest_version: None,
                    latest_tag_name: None,
                    upgrade_available: false,
                    install_prefix: Some(context.install_prefix.display().to_string()),
                    current_release_root: Some(context.release_root.display().to_string()),
                    repository: context.repository.clone(),
                    latest_release_url: Some(context.latest_release_url()),
                    download_url: None,
                    reason: Some(error.to_string()),
                });
            }
        };

        let current_version = Version::parse(&context.current_version).with_context(|| {
            format!(
                "invalid installed release version: {}",
                context.current_version
            )
        })?;
        let latest_version =
            Version::parse(&release.metadata.release_version).with_context(|| {
                format!(
                    "invalid remote release version: {}",
                    release.metadata.release_version
                )
            })?;

        Ok(UpgradeStatus {
            supported: true,
            current_version: Some(context.current_version.clone()),
            latest_version: Some(release.metadata.release_version.clone()),
            latest_tag_name: Some(release.metadata.tag_name.clone()),
            upgrade_available: latest_version > current_version,
            install_prefix: Some(context.install_prefix.display().to_string()),
            current_release_root: Some(context.release_root.display().to_string()),
            repository: context.repository.clone(),
            latest_release_url: Some(context.latest_release_url()),
            download_url: Some(release.bundle_url.clone()),
            reason: None,
        })
    }

    async fn apply(&self) -> Result<UpgradeApplyResult> {
        let context = load_install_context()?.ok_or_else(|| {
            anyhow!("self-upgrade is only available for installed release bundles")
        })?;
        let release = resolve_release(&self.client, &context).await?;

        let current_version = Version::parse(&context.current_version).with_context(|| {
            format!(
                "invalid installed release version: {}",
                context.current_version
            )
        })?;
        let target_version =
            Version::parse(&release.metadata.release_version).with_context(|| {
                format!(
                    "invalid remote release version: {}",
                    release.metadata.release_version
                )
            })?;

        let status = UpgradeStatus {
            supported: true,
            current_version: Some(context.current_version.clone()),
            latest_version: Some(release.metadata.release_version.clone()),
            latest_tag_name: Some(release.metadata.tag_name.clone()),
            upgrade_available: target_version > current_version,
            install_prefix: Some(context.install_prefix.display().to_string()),
            current_release_root: Some(context.release_root.display().to_string()),
            repository: context.repository.clone(),
            latest_release_url: Some(context.latest_release_url()),
            download_url: Some(release.bundle_url.clone()),
            reason: None,
        };

        if target_version <= current_version {
            return Ok(UpgradeApplyResult {
                ok: true,
                action: "noop",
                current_version: Some(context.current_version.clone()),
                target_version: Some(release.metadata.release_version.clone()),
                launcher_path: Some(context.launcher_path().display().to_string()),
                installed_release_root: Some(context.release_root.display().to_string()),
                relaunch_required: false,
                message: "already running the latest installed release".to_string(),
                status,
            });
        }

        let bundle_bytes = download_bytes(&self.client, &release.bundle_url).await?;
        verify_sha256(&bundle_bytes, &release.bundle_checksum)?;

        let bundle_manifest_bytes =
            download_bytes(&self.client, &release.bundle_manifest_url).await?;
        let bundle_manifest: BundleManifest = serde_json::from_slice(&bundle_manifest_bytes)
            .context("failed to parse bundle manifest")?;
        if bundle_manifest.release_version != release.metadata.release_version {
            bail!(
                "bundle manifest release version mismatch: {} != {}",
                bundle_manifest.release_version,
                release.metadata.release_version
            );
        }

        let temp_dir = TempDir::new().context("failed to create upgrade temp dir")?;
        let unpack_dir = unpack_bundle(&bundle_bytes, temp_dir.path())?;
        let install_script = unpack_dir.join("install.sh");
        if !install_script.exists() {
            bail!("install.sh missing from downloaded bundle");
        }

        let output = Command::new("bash")
            .arg(&install_script)
            .arg("--prefix")
            .arg(&context.install_prefix)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("failed to execute install.sh")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!(
                "install.sh failed with status {}: {}{}",
                output.status,
                stdout,
                stderr
            );
        }

        let installed_release_root = context
            .install_prefix
            .join("share/chaos-bot/releases")
            .join(&release.metadata.release_version);

        Ok(UpgradeApplyResult {
            ok: true,
            action: "upgrade",
            current_version: Some(context.current_version.clone()),
            target_version: Some(release.metadata.release_version.clone()),
            launcher_path: Some(context.launcher_path().display().to_string()),
            installed_release_root: Some(installed_release_root.display().to_string()),
            relaunch_required: true,
            message: format!(
                "installed {}. relaunch the launcher to start the new release",
                release.metadata.release_version
            ),
            status,
        })
    }

    async fn relaunch(&self) -> Result<UpgradeRestartResult> {
        let context = load_install_context()?.ok_or_else(|| {
            anyhow!("self-upgrade restart is only available for installed release bundles")
        })?;

        let launcher_path = context.launcher_path();
        if !launcher_path.exists() {
            bail!("launcher not found: {}", launcher_path.display());
        }

        Command::new("bash")
            .arg("-lc")
            .arg("sleep 1; exec \"$1\"")
            .arg("--")
            .arg(&launcher_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("failed to spawn launcher restart helper")?;

        tokio::spawn(async {
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            std::process::exit(0);
        });

        Ok(UpgradeRestartResult {
            ok: true,
            action: "relaunch",
            launcher_path: Some(launcher_path.display().to_string()),
            target_version: Some(context.current_version.clone()),
            message: format!("restart scheduled via {}", launcher_path.display()),
        })
    }
}

#[derive(Clone, Debug)]
struct InstallContext {
    current_version: String,
    install_prefix: PathBuf,
    release_root: PathBuf,
    repository: Option<String>,
    github_api_base_url: String,
    latest_release_url_override: Option<String>,
}

impl InstallContext {
    fn latest_release_url(&self) -> String {
        if let Some(url) = &self.latest_release_url_override {
            return url.clone();
        }
        let repository = self.repository.clone().unwrap_or_default();
        format!(
            "{}/repos/{}/releases/latest",
            self.github_api_base_url.trim_end_matches('/'),
            repository
        )
    }

    fn launcher_path(&self) -> PathBuf {
        self.install_prefix.join("bin/chaos-bot")
    }
}

#[derive(Clone, Debug, Deserialize)]
struct InstalledReleaseManifest {
    release_version: String,
    repository: Option<String>,
    release_api_base_url: Option<String>,
    latest_release_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ReleaseMetadata {
    release_version: String,
    tag_name: String,
    artifact_stem: String,
}

#[derive(Clone, Debug)]
struct ResolvedRelease {
    metadata: ReleaseMetadata,
    bundle_url: String,
    bundle_manifest_url: String,
    bundle_checksum: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Clone, Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone, Debug, Deserialize)]
struct BundleManifest {
    release_version: String,
}

fn unsupported_status(reason: &str) -> UpgradeStatus {
    UpgradeStatus {
        supported: false,
        current_version: None,
        latest_version: None,
        latest_tag_name: None,
        upgrade_available: false,
        install_prefix: None,
        current_release_root: None,
        repository: None,
        latest_release_url: None,
        download_url: None,
        reason: Some(reason.to_string()),
    }
}

fn load_install_context() -> Result<Option<InstallContext>> {
    let release_root = resolve_release_root()?;
    let Some(release_root) = release_root else {
        return Ok(None);
    };

    let manifest_path = release_root.join("release-manifest.json");
    let manifest = if manifest_path.exists() {
        Some(
            serde_json::from_slice::<InstalledReleaseManifest>(&fs::read(&manifest_path)?)
                .with_context(|| format!("failed to parse {}", manifest_path.display()))?,
        )
    } else {
        None
    };

    let current_version = env::var("CHAOS_BOT_RELEASE_VERSION")
        .ok()
        .or_else(|| manifest.as_ref().map(|item| item.release_version.clone()));
    let Some(current_version) = current_version else {
        return Ok(None);
    };

    let install_prefix = if let Ok(value) = env::var("CHAOS_BOT_INSTALL_PREFIX") {
        PathBuf::from(value)
    } else {
        infer_install_prefix(&release_root)?
    };

    let repository = env::var("CHAOS_BOT_UPGRADE_REPOSITORY")
        .ok()
        .or_else(|| manifest.as_ref().and_then(|item| item.repository.clone()));
    let github_api_base_url = env::var("CHAOS_BOT_UPGRADE_API_BASE_URL")
        .ok()
        .or_else(|| {
            manifest
                .as_ref()
                .and_then(|item| item.release_api_base_url.clone())
        })
        .unwrap_or_else(|| DEFAULT_GITHUB_API_BASE_URL.to_string());
    let latest_release_url_override = env::var("CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL")
        .ok()
        .or_else(|| {
            manifest
                .as_ref()
                .and_then(|item| item.latest_release_url.clone())
        });

    if latest_release_url_override.is_none() && repository.is_none() {
        return Ok(None);
    }

    Ok(Some(InstallContext {
        current_version,
        install_prefix,
        release_root,
        repository,
        github_api_base_url,
        latest_release_url_override,
    }))
}

fn resolve_release_root() -> Result<Option<PathBuf>> {
    if let Ok(value) = env::var("CHAOS_BOT_RELEASE_ROOT") {
        let path = PathBuf::from(value);
        if path.exists() {
            return Ok(Some(path));
        }
    }

    let exe = env::current_exe().context("failed to inspect current executable")?;
    let Some(bin_dir) = exe.parent() else {
        return Ok(None);
    };
    let Some(release_root) = bin_dir.parent() else {
        return Ok(None);
    };
    let manifest_path = release_root.join("release-manifest.json");
    if manifest_path.exists() {
        Ok(Some(release_root.to_path_buf()))
    } else {
        Ok(None)
    }
}

fn infer_install_prefix(release_root: &Path) -> Result<PathBuf> {
    release_root
        .ancestors()
        .nth(4)
        .map(|path| path.to_path_buf())
        .ok_or_else(|| {
            anyhow!(
                "failed to infer install prefix from {}",
                release_root.display()
            )
        })
}

async fn resolve_release(client: &Client, context: &InstallContext) -> Result<ResolvedRelease> {
    let latest_release: GitHubRelease = client
        .get(context.latest_release_url())
        .send()
        .await
        .context("failed to fetch latest release descriptor")?
        .error_for_status()
        .context("latest release request returned error status")?
        .json()
        .await
        .context("failed to decode latest release descriptor")?;

    let metadata_asset = asset_url(&latest_release.assets, "release-metadata.json")?;
    let metadata_checksum_asset = asset_url(&latest_release.assets, "release-metadata.sha256")?;

    let metadata_bytes = download_bytes(client, &metadata_asset).await?;
    let metadata_checksum = download_text(client, &metadata_checksum_asset).await?;
    verify_sha256(&metadata_bytes, &metadata_checksum)?;

    let metadata: ReleaseMetadata =
        serde_json::from_slice(&metadata_bytes).context("failed to parse release metadata")?;
    if metadata.tag_name != latest_release.tag_name {
        bail!(
            "release metadata tag mismatch: {} != {}",
            metadata.tag_name,
            latest_release.tag_name
        );
    }

    let bundle_name = format!(
        "{}-{}.tar.gz",
        metadata.artifact_stem, DEFAULT_BUNDLE_TARGET
    );
    let bundle_checksum_name = format!("{bundle_name}.sha256");
    let bundle_manifest_name = format!(
        "{}-{}.manifest.json",
        metadata.artifact_stem, DEFAULT_BUNDLE_TARGET
    );

    Ok(ResolvedRelease {
        metadata,
        bundle_url: asset_url(&latest_release.assets, &bundle_name)?,
        bundle_manifest_url: asset_url(&latest_release.assets, &bundle_manifest_name)?,
        bundle_checksum: download_text(
            client,
            &asset_url(&latest_release.assets, &bundle_checksum_name)?,
        )
        .await?,
    })
}

fn asset_url(assets: &[GitHubReleaseAsset], name: &str) -> Result<String> {
    assets
        .iter()
        .find(|asset| asset.name == name)
        .map(|asset| asset.browser_download_url.clone())
        .ok_or_else(|| anyhow!("release asset not found: {name}"))
}

async fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download {url}"))?
        .error_for_status()
        .with_context(|| format!("download returned error status: {url}"))?
        .bytes()
        .await
        .with_context(|| format!("failed to read download body: {url}"))
        .map(|bytes| bytes.to_vec())
}

async fn download_text(client: &Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .with_context(|| format!("failed to download {url}"))?
        .error_for_status()
        .with_context(|| format!("download returned error status: {url}"))?
        .text()
        .await
        .with_context(|| format!("failed to read text body: {url}"))
}

fn verify_sha256(bytes: &[u8], checksum_file: &str) -> Result<()> {
    let expected = checksum_file
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("checksum file did not contain a digest"))?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        bail!("checksum mismatch: expected {expected}, got {actual}");
    }
    Ok(())
}

fn unpack_bundle(bytes: &[u8], parent_dir: &Path) -> Result<PathBuf> {
    let archive_root = parent_dir.join("bundle");
    fs::create_dir_all(&archive_root)?;
    let decoder = GzDecoder::new(bytes);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(&archive_root)
        .context("failed to unpack release bundle")?;

    let mut entries = fs::read_dir(&archive_root)
        .with_context(|| format!("failed to read {}", archive_root.display()))?;
    let first = entries
        .next()
        .ok_or_else(|| anyhow!("bundle archive was empty"))?
        .context("failed to inspect unpacked bundle")?;
    Ok(first.path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::Path as AxumPath;
    use axum::http::StatusCode;
    use axum::routing::get;
    use axum::{response::IntoResponse, Json, Router};
    use serde_json::json;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::net::TcpListener;

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: impl AsRef<str>) -> Self {
            let original = env::var(key).ok();
            env::set_var(key, value.as_ref());
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                env::set_var(self.key, value);
            } else {
                env::remove_var(self.key);
            }
        }
    }

    fn clear_upgrade_env() {
        for key in [
            "CHAOS_BOT_RELEASE_ROOT",
            "CHAOS_BOT_RELEASE_VERSION",
            "CHAOS_BOT_INSTALL_PREFIX",
            "CHAOS_BOT_UPGRADE_REPOSITORY",
            "CHAOS_BOT_UPGRADE_API_BASE_URL",
            "CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL",
        ] {
            env::remove_var(key);
        }
    }

    fn build_bundle(version: &str) -> Vec<u8> {
        let temp = tempdir().expect("tempdir");
        let bundle_root = temp.path().join(format!("{version}-linux-x86_64"));
        fs::create_dir_all(bundle_root.join("bin")).expect("create bin");

        fs::write(
            bundle_root.join("release-manifest.json"),
            json!({
                "release_version": version,
                "repository": "test/chaos-bot",
                "release_api_base_url": "https://api.github.com",
            })
            .to_string(),
        )
        .expect("write manifest");

        fs::write(
            bundle_root.join("install.sh"),
            format!(
                r#"#!/usr/bin/env bash
set -euo pipefail
PREFIX="$HOME/.local"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --prefix)
      PREFIX="$2"
      shift 2
      ;;
    *)
      exit 1
      ;;
  esac
done
RELEASE_ROOT="${{PREFIX}}/share/chaos-bot/releases/{version}"
mkdir -p "${{RELEASE_ROOT}}/bin" "${{PREFIX}}/bin"
cp "${{BASH_SOURCE[0]%/*}}/release-manifest.json" "${{RELEASE_ROOT}}/release-manifest.json"
cat > "${{PREFIX}}/bin/chaos-bot" <<'LAUNCHER'
#!/usr/bin/env bash
set -euo pipefail
echo {version}
LAUNCHER
chmod +x "${{PREFIX}}/bin/chaos-bot"
"#
            ),
        )
        .expect("write install script");

        let tar_gz = temp.path().join("bundle.tar.gz");
        let tar_file = fs::File::create(&tar_gz).expect("create tar");
        let encoder = flate2::write::GzEncoder::new(tar_file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        builder
            .append_dir_all(format!("{version}-linux-x86_64"), &bundle_root)
            .expect("append bundle dir");
        let encoder = builder.into_inner().expect("finish tar");
        encoder.finish().expect("finish gzip");
        fs::read(tar_gz).expect("read tarball")
    }

    fn setup_installed_release(
        temp: &tempfile::TempDir,
        version: &str,
        latest_url: &str,
    ) -> Vec<EnvGuard> {
        let install_prefix = temp.path().join("install");
        let release_root = install_prefix
            .join("share/chaos-bot/releases")
            .join(version);
        fs::create_dir_all(&release_root).expect("create release root");
        fs::write(
            release_root.join("release-manifest.json"),
            json!({
                "release_version": version,
                "repository": "test/chaos-bot",
                "release_api_base_url": "http://127.0.0.1:0",
                "latest_release_url": latest_url,
            })
            .to_string(),
        )
        .expect("write installed manifest");

        vec![
            EnvGuard::set("CHAOS_BOT_RELEASE_ROOT", release_root.display().to_string()),
            EnvGuard::set("CHAOS_BOT_RELEASE_VERSION", version),
            EnvGuard::set(
                "CHAOS_BOT_INSTALL_PREFIX",
                install_prefix.display().to_string(),
            ),
            EnvGuard::set("CHAOS_BOT_UPGRADE_LATEST_RELEASE_URL", latest_url),
        ]
    }

    async fn serve_assets(
        latest_release: serde_json::Value,
        assets: HashMap<String, Vec<u8>>,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let base_url = format!("http://127.0.0.1:{}", addr.port());
        let latest_release = serde_json::to_string(&latest_release)
            .expect("latest release json")
            .replace("BASE", &base_url);
        let latest_release: serde_json::Value =
            serde_json::from_str(&latest_release).expect("latest release value");
        let assets = Arc::new(assets);
        let latest_release = Arc::new(latest_release);
        let app = Router::new()
            .route(
                "/repos/test/chaos-bot/releases/latest",
                get({
                    let latest_release = latest_release.clone();
                    move || {
                        let latest_release = latest_release.clone();
                        async move { Json((*latest_release).clone()) }
                    }
                }),
            )
            .route(
                "/assets/:name",
                get({
                    let assets = assets.clone();
                    move |AxumPath(name): AxumPath<String>| {
                        let assets = assets.clone();
                        async move {
                            match assets.get(&name) {
                                Some(bytes) => (StatusCode::OK, bytes.clone()).into_response(),
                                None => StatusCode::NOT_FOUND.into_response(),
                            }
                        }
                    }
                }),
            );
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (base_url, handle)
    }

    fn checksum_for(bytes: &[u8], name: &str) -> Vec<u8> {
        format!("{:x}  {name}\n", Sha256::digest(bytes)).into_bytes()
    }

    #[tokio::test]
    #[serial]
    async fn status_reports_upgrade_availability() {
        clear_upgrade_env();
        let temp = tempdir().expect("tempdir");
        let bundle = build_bundle("0.1.1");
        let metadata = json!({
            "release_version": "0.1.1",
            "tag_name": "v0.1.1",
            "artifact_stem": "0.1.1"
        });
        let metadata_bytes = serde_json::to_vec(&metadata).expect("metadata bytes");
        let manifest_bytes = json!({ "release_version": "0.1.1" })
            .to_string()
            .into_bytes();

        let (base_url, _server) = serve_assets(
            json!({
                "tag_name": "v0.1.1",
                "assets": [
                    { "name": "release-metadata.json", "browser_download_url": "BASE/assets/release-metadata.json" },
                    { "name": "release-metadata.sha256", "browser_download_url": "BASE/assets/release-metadata.sha256" },
                    { "name": "0.1.1-linux-x86_64.tar.gz", "browser_download_url": "BASE/assets/0.1.1-linux-x86_64.tar.gz" },
                    { "name": "0.1.1-linux-x86_64.tar.gz.sha256", "browser_download_url": "BASE/assets/0.1.1-linux-x86_64.tar.gz.sha256" },
                    { "name": "0.1.1-linux-x86_64.manifest.json", "browser_download_url": "BASE/assets/0.1.1-linux-x86_64.manifest.json" }
                ]
            }),
            HashMap::from([
                ("release-metadata.json".to_string(), metadata_bytes.clone()),
                ("release-metadata.sha256".to_string(), checksum_for(&metadata_bytes, "release-metadata.json")),
                ("0.1.1-linux-x86_64.tar.gz".to_string(), bundle.clone()),
                (
                    "0.1.1-linux-x86_64.tar.gz.sha256".to_string(),
                    checksum_for(&bundle, "0.1.1-linux-x86_64.tar.gz"),
                ),
                ("0.1.1-linux-x86_64.manifest.json".to_string(), manifest_bytes),
            ]),
        )
        .await;

        let latest_url = format!("{base_url}/repos/test/chaos-bot/releases/latest");
        let _envs = setup_installed_release(&temp, "0.1.0", &latest_url);

        let updater = GitHubReleaseUpdater::new().expect("updater");
        let status = updater.status().await.expect("status");
        assert!(status.supported);
        assert!(status.upgrade_available);
        assert_eq!(status.current_version.as_deref(), Some("0.1.0"));
        assert_eq!(status.latest_version.as_deref(), Some("0.1.1"));
    }

    #[tokio::test]
    #[serial]
    async fn apply_noops_when_versions_match() {
        clear_upgrade_env();
        let temp = tempdir().expect("tempdir");
        let bundle = build_bundle("0.1.1");
        let metadata = json!({
            "release_version": "0.1.1",
            "tag_name": "v0.1.1",
            "artifact_stem": "0.1.1"
        });
        let metadata_bytes = serde_json::to_vec(&metadata).expect("metadata bytes");
        let manifest_bytes = json!({ "release_version": "0.1.1" })
            .to_string()
            .into_bytes();

        let (base_url, _server) = serve_assets(
            json!({
                "tag_name": "v0.1.1",
                "assets": [
                    { "name": "release-metadata.json", "browser_download_url": "BASE/assets/release-metadata.json" },
                    { "name": "release-metadata.sha256", "browser_download_url": "BASE/assets/release-metadata.sha256" },
                    { "name": "0.1.1-linux-x86_64.tar.gz", "browser_download_url": "BASE/assets/0.1.1-linux-x86_64.tar.gz" },
                    { "name": "0.1.1-linux-x86_64.tar.gz.sha256", "browser_download_url": "BASE/assets/0.1.1-linux-x86_64.tar.gz.sha256" },
                    { "name": "0.1.1-linux-x86_64.manifest.json", "browser_download_url": "BASE/assets/0.1.1-linux-x86_64.manifest.json" }
                ]
            }),
            HashMap::from([
                ("release-metadata.json".to_string(), metadata_bytes.clone()),
                ("release-metadata.sha256".to_string(), checksum_for(&metadata_bytes, "release-metadata.json")),
                ("0.1.1-linux-x86_64.tar.gz".to_string(), bundle.clone()),
                (
                    "0.1.1-linux-x86_64.tar.gz.sha256".to_string(),
                    checksum_for(&bundle, "0.1.1-linux-x86_64.tar.gz"),
                ),
                ("0.1.1-linux-x86_64.manifest.json".to_string(), manifest_bytes),
            ]),
        )
        .await;

        let latest_url = format!("{base_url}/repos/test/chaos-bot/releases/latest");
        let _envs = setup_installed_release(&temp, "0.1.1", &latest_url);

        let updater = GitHubReleaseUpdater::new().expect("updater");
        let result = updater.apply().await.expect("apply");
        assert_eq!(result.action, "noop");
        assert!(!result.relaunch_required);
    }

    #[tokio::test]
    #[serial]
    async fn apply_installs_new_release() {
        clear_upgrade_env();
        let temp = tempdir().expect("tempdir");
        let bundle = build_bundle("0.1.2");
        let metadata = json!({
            "release_version": "0.1.2",
            "tag_name": "v0.1.2",
            "artifact_stem": "0.1.2"
        });
        let metadata_bytes = serde_json::to_vec(&metadata).expect("metadata bytes");
        let manifest_bytes = json!({ "release_version": "0.1.2" })
            .to_string()
            .into_bytes();

        let (base_url, _server) = serve_assets(
            json!({
                "tag_name": "v0.1.2",
                "assets": [
                    { "name": "release-metadata.json", "browser_download_url": "BASE/assets/release-metadata.json" },
                    { "name": "release-metadata.sha256", "browser_download_url": "BASE/assets/release-metadata.sha256" },
                    { "name": "0.1.2-linux-x86_64.tar.gz", "browser_download_url": "BASE/assets/0.1.2-linux-x86_64.tar.gz" },
                    { "name": "0.1.2-linux-x86_64.tar.gz.sha256", "browser_download_url": "BASE/assets/0.1.2-linux-x86_64.tar.gz.sha256" },
                    { "name": "0.1.2-linux-x86_64.manifest.json", "browser_download_url": "BASE/assets/0.1.2-linux-x86_64.manifest.json" }
                ]
            }),
            HashMap::from([
                ("release-metadata.json".to_string(), metadata_bytes.clone()),
                ("release-metadata.sha256".to_string(), checksum_for(&metadata_bytes, "release-metadata.json")),
                ("0.1.2-linux-x86_64.tar.gz".to_string(), bundle.clone()),
                (
                    "0.1.2-linux-x86_64.tar.gz.sha256".to_string(),
                    checksum_for(&bundle, "0.1.2-linux-x86_64.tar.gz"),
                ),
                ("0.1.2-linux-x86_64.manifest.json".to_string(), manifest_bytes),
            ]),
        )
        .await;

        let latest_url = format!("{base_url}/repos/test/chaos-bot/releases/latest");
        let _envs = setup_installed_release(&temp, "0.1.1", &latest_url);

        let updater = GitHubReleaseUpdater::new().expect("updater");
        let result = updater.apply().await.expect("apply");
        assert_eq!(result.action, "upgrade");
        assert!(result.relaunch_required);

        let install_prefix = temp.path().join("install");
        let launcher = install_prefix.join("bin/chaos-bot");
        assert!(launcher.exists());
        let release_manifest =
            install_prefix.join("share/chaos-bot/releases/0.1.2/release-manifest.json");
        assert!(release_manifest.exists());
    }

    #[tokio::test]
    #[serial]
    async fn status_reports_checksum_failure_reason() {
        clear_upgrade_env();
        let temp = tempdir().expect("tempdir");
        let bundle = build_bundle("0.1.3");
        let metadata = json!({
            "release_version": "0.1.3",
            "tag_name": "v0.1.3",
            "artifact_stem": "0.1.3"
        });
        let metadata_bytes = serde_json::to_vec(&metadata).expect("metadata bytes");

        let (base_url, _server) = serve_assets(
            json!({
                "tag_name": "v0.1.3",
                "assets": [
                    { "name": "release-metadata.json", "browser_download_url": "BASE/assets/release-metadata.json" },
                    { "name": "release-metadata.sha256", "browser_download_url": "BASE/assets/release-metadata.sha256" },
                    { "name": "0.1.3-linux-x86_64.tar.gz", "browser_download_url": "BASE/assets/0.1.3-linux-x86_64.tar.gz" },
                    { "name": "0.1.3-linux-x86_64.tar.gz.sha256", "browser_download_url": "BASE/assets/0.1.3-linux-x86_64.tar.gz.sha256" },
                    { "name": "0.1.3-linux-x86_64.manifest.json", "browser_download_url": "BASE/assets/0.1.3-linux-x86_64.manifest.json" }
                ]
            }),
            HashMap::from([
                ("release-metadata.json".to_string(), metadata_bytes.clone()),
                ("release-metadata.sha256".to_string(), b"deadbeef  release-metadata.json\n".to_vec()),
                ("0.1.3-linux-x86_64.tar.gz".to_string(), bundle),
                ("0.1.3-linux-x86_64.tar.gz.sha256".to_string(), b"deadbeef  bundle.tar.gz\n".to_vec()),
                ("0.1.3-linux-x86_64.manifest.json".to_string(), br#"{"release_version":"0.1.3"}"#.to_vec()),
            ]),
        )
        .await;

        let latest_url = format!("{base_url}/repos/test/chaos-bot/releases/latest");
        let _envs = setup_installed_release(&temp, "0.1.2", &latest_url);

        let updater = GitHubReleaseUpdater::new().expect("updater");
        let status = updater.status().await.expect("status");
        assert!(status.supported);
        assert!(!status.upgrade_available);
        assert!(status
            .reason
            .unwrap_or_default()
            .contains("checksum mismatch"));
    }
}
