pub mod claude;
pub mod codex;
pub mod gemini;

use std::path::PathBuf;

/// Result of a hook installation attempt.
#[derive(Debug)]
pub struct InstallResult {
    pub agent: String,
    pub config_path: PathBuf,
    pub status: InstallStatus,
}

#[derive(Debug)]
pub enum InstallStatus {
    Installed,
    AlreadyInstalled,
    ConfigNotFound,
    Failed(String),
}

impl std::fmt::Display for InstallResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let icon = match &self.status {
            InstallStatus::Installed => "OK",
            InstallStatus::AlreadyInstalled => "SKIP",
            InstallStatus::ConfigNotFound => "MISS",
            InstallStatus::Failed(_) => "FAIL",
        };
        let detail = match &self.status {
            InstallStatus::Installed => "hook installed".to_string(),
            InstallStatus::AlreadyInstalled => "already installed".to_string(),
            InstallStatus::ConfigNotFound => "config not found".to_string(),
            InstallStatus::Failed(e) => format!("error: {e}"),
        };
        write!(
            f,
            "  [{icon}] {}: {} ({})",
            self.agent,
            detail,
            self.config_path.display()
        )
    }
}

/// Install hooks for all supported agents.
/// Returns results for each agent (never fails entirely).
#[must_use]
pub fn install_all() -> Vec<InstallResult> {
    vec![claude::install(), codex::install(), gemini::install()]
}

/// Uninstall hooks from all supported agents.
#[must_use]
pub fn uninstall_all() -> Vec<InstallResult> {
    vec![claude::uninstall(), codex::uninstall(), gemini::uninstall()]
}
