use std::path::PathBuf;

use super::{AgentHookConfig, InstallResult, generic_install, generic_uninstall};

fn config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        let xdg_path = config_dir.join("codex").join("config.json");
        if xdg_path.exists() {
            return xdg_path;
        }
    }

    dirs::home_dir()
        .unwrap_or_default()
        .join(".codex")
        .join("config.json")
}

#[must_use]
pub fn install() -> InstallResult {
    generic_install(AgentHookConfig {
        agent_name: "Codex CLI".to_string(),
        config_path: config_path(),
        hooks_key: "hooks".to_string(),
        entries_key: "post_exec".to_string(),
        hook_entries: vec![serde_json::json!({
            "name": "cli-denoiser",
            "command": "cli-denoiser --hook-mode",
            "on": ["shell"]
        })],
    })
}

#[must_use]
pub fn uninstall() -> InstallResult {
    generic_uninstall("Codex CLI".to_string(), config_path(), "/hooks/post_exec")
}
