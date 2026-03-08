use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct UpgradeStatus {
    pub supported: bool,
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    pub latest_tag_name: Option<String>,
    pub upgrade_available: bool,
    pub install_prefix: Option<String>,
    pub current_release_root: Option<String>,
    pub repository: Option<String>,
    pub latest_release_url: Option<String>,
    pub download_url: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpgradeApplyResult {
    pub ok: bool,
    pub action: &'static str,
    pub current_version: Option<String>,
    pub target_version: Option<String>,
    pub launcher_path: Option<String>,
    pub installed_release_root: Option<String>,
    pub relaunch_required: bool,
    pub message: String,
    pub status: UpgradeStatus,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpgradeRestartResult {
    pub ok: bool,
    pub action: &'static str,
    pub launcher_path: Option<String>,
    pub target_version: Option<String>,
    pub message: String,
}
