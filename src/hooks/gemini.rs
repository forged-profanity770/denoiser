use std::path::PathBuf;

use super::{AgentHookConfig, InstallResult, generic_install, generic_uninstall};

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".gemini")
        .join("settings.json")
}

#[must_use]
pub fn install() -> InstallResult {
    generic_install(AgentHookConfig {
        agent_name: "Gemini CLI".to_string(),
        config_path: config_path(),
        hooks_key: "hooks".to_string(),
        entries_key: "post_tool_use".to_string(),
        hook_entries: vec![serde_json::json!({
            "name": "cli-denoiser",
            "command": "cli-denoiser --hook-mode",
            "tools": ["shell", "bash"]
        })],
    })
}

#[must_use]
pub fn uninstall() -> InstallResult {
    generic_uninstall(
        "Gemini CLI".to_string(),
        config_path(),
        "/hooks/post_tool_use",
    )
}
