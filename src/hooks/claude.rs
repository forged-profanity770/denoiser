use std::path::PathBuf;

use super::{AgentHookConfig, InstallResult, generic_install, generic_uninstall};

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("settings.json")
}

#[must_use]
pub fn install() -> InstallResult {
    generic_install(AgentHookConfig {
        agent_name: "Claude Code".to_string(),
        config_path: config_path(),
        hooks_key: "hooks".to_string(),
        entries_key: "PostToolUse".to_string(),
        hook_entries: vec![
            serde_json::json!({
                "matcher": "Bash",
                "hooks": [{
                    "type": "command",
                    "command": "cli-denoiser --hook-mode",
                    "description": "cli-denoiser: filter terminal noise from Bash output"
                }]
            }),
            serde_json::json!({
                "matcher": "Read",
                "hooks": [{
                    "type": "command",
                    "command": "cli-denoiser --hook-mode",
                    "description": "cli-denoiser: filter noise from file reads"
                }]
            }),
            serde_json::json!({
                "matcher": "Grep",
                "hooks": [{
                    "type": "command",
                    "command": "cli-denoiser --hook-mode",
                    "description": "cli-denoiser: filter noise from grep results"
                }]
            }),
        ],
    })
}

#[must_use]
pub fn uninstall() -> InstallResult {
    generic_uninstall(
        "Claude Code".to_string(),
        config_path(),
        "/hooks/PostToolUse",
    )
}
