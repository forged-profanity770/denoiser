use std::path::PathBuf;

use super::{InstallResult, InstallStatus};

const HOOK_MARKER: &str = "cli-denoiser";

/// Claude Code stores settings in ~/.claude/settings.json
/// Hook format: hooks.PostToolUse that pipes Bash output through cli-denoiser
fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".claude")
        .join("settings.json")
}

#[must_use]
pub fn install() -> InstallResult {
    let path = config_path();
    let agent = "Claude Code".to_string();

    if !path.exists() {
        return InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::ConfigNotFound,
        };
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return InstallResult {
                agent,
                config_path: path,
                status: InstallStatus::Failed(e.to_string()),
            };
        }
    };

    if content.contains(HOOK_MARKER) {
        return InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::AlreadyInstalled,
        };
    }

    let mut settings: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return InstallResult {
                agent,
                config_path: path,
                status: InstallStatus::Failed(format!("invalid JSON: {e}")),
            };
        }
    };

    // Add PostToolUse hook for Bash tool
    let hooks = settings.as_object_mut().and_then(|obj| {
        obj.entry("hooks")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
    });

    let Some(hooks) = hooks else {
        return InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::Failed("could not access hooks object".to_string()),
        };
    };

    let post_tool = hooks
        .entry("PostToolUse")
        .or_insert_with(|| serde_json::json!([]));

    let hook_entry = serde_json::json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": "cli-denoiser --hook-mode",
            "description": "cli-denoiser: filter terminal noise from Bash output"
        }]
    });

    if let Some(arr) = post_tool.as_array_mut() {
        arr.push(hook_entry);
    }

    match write_json(&path, &settings) {
        Ok(()) => InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::Installed,
        },
        Err(e) => InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::Failed(e),
        },
    }
}

#[must_use]
pub fn uninstall() -> InstallResult {
    let path = config_path();
    let agent = "Claude Code".to_string();

    if !path.exists() {
        return InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::ConfigNotFound,
        };
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return InstallResult {
                agent,
                config_path: path,
                status: InstallStatus::Failed(e.to_string()),
            };
        }
    };

    if !content.contains(HOOK_MARKER) {
        return InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::ConfigNotFound,
        };
    }

    let mut settings: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return InstallResult {
                agent,
                config_path: path,
                status: InstallStatus::Failed(format!("invalid JSON: {e}")),
            };
        }
    };

    // Remove our hook entries
    if let Some(hooks) = settings.pointer_mut("/hooks/PostToolUse")
        && let Some(arr) = hooks.as_array_mut()
    {
        arr.retain(|entry| {
            let s = entry.to_string();
            !s.contains(HOOK_MARKER)
        });
    }

    match write_json(&path, &settings) {
        Ok(()) => InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::Installed,
        },
        Err(e) => InstallResult {
            agent,
            config_path: path,
            status: InstallStatus::Failed(e),
        },
    }
}

fn write_json(path: &std::path::Path, value: &serde_json::Value) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
