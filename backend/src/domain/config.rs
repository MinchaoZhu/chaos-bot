use crate::infrastructure::config::AgentFileConfig;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ConfigStateResponse {
    pub config_path: String,
    pub backup1_path: String,
    pub backup2_path: String,
    pub config_format: String,
    pub running: AgentFileConfig,
    pub disk: AgentFileConfig,
    pub raw: String,
    pub disk_parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigMutationResponse {
    pub ok: bool,
    pub action: &'static str,
    pub restart_scheduled: bool,
    pub state: ConfigStateResponse,
}

#[derive(Debug, Clone)]
pub enum ConfigMutationInput {
    Raw(String),
    Structured(AgentFileConfig),
}

#[derive(Debug, Clone)]
pub enum ConfigRestartInput {
    Noop,
    Raw(String),
    Structured(AgentFileConfig),
}
