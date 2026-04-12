pub mod claude;
pub mod codex;
pub mod gemini;

use std::path::PathBuf;

const HOOK_MARKER: &str = "cli-denoiser";

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

/// Per-agent hook configuration.
struct AgentHookConfig {
    agent_name: String,
    config_path: PathBuf,
    hooks_key: String,
    entries_key: String,
    hook_entries: Vec<serde_json::Value>,
}

fn generic_install(config: AgentHookConfig) -> InstallResult {
    let AgentHookConfig {
        agent_name,
        config_path,
        hooks_key,
        entries_key,
        hook_entries,
    } = config;

    if !config_path.exists() {
        return InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::ConfigNotFound,
        };
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return InstallResult {
                agent: agent_name,
                config_path,
                status: InstallStatus::Failed(e.to_string()),
            };
        }
    };

    if content.contains(HOOK_MARKER) {
        return InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::AlreadyInstalled,
        };
    }

    let mut settings: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return InstallResult {
                agent: agent_name,
                config_path,
                status: InstallStatus::Failed(format!("invalid JSON: {e}")),
            };
        }
    };

    let hooks = settings.as_object_mut().and_then(|obj| {
        obj.entry(&hooks_key)
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
    });

    let Some(hooks) = hooks else {
        return InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::Failed("could not access hooks object".to_string()),
        };
    };

    let entries = hooks
        .entry(&entries_key)
        .or_insert_with(|| serde_json::json!([]));

    if let Some(arr) = entries.as_array_mut() {
        for entry in hook_entries {
            arr.push(entry);
        }
    }

    match write_json(&config_path, &settings) {
        Ok(()) => InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::Installed,
        },
        Err(e) => InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::Failed(e),
        },
    }
}

fn generic_uninstall(agent_name: String, config_path: PathBuf, pointer: &str) -> InstallResult {
    if !config_path.exists() {
        return InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::ConfigNotFound,
        };
    }

    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return InstallResult {
                agent: agent_name,
                config_path,
                status: InstallStatus::Failed(e.to_string()),
            };
        }
    };

    if !content.contains(HOOK_MARKER) {
        return InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::ConfigNotFound,
        };
    }

    let mut settings: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            return InstallResult {
                agent: agent_name,
                config_path,
                status: InstallStatus::Failed(format!("invalid JSON: {e}")),
            };
        }
    };

    if let Some(hooks) = settings.pointer_mut(pointer)
        && let Some(arr) = hooks.as_array_mut()
    {
        arr.retain(|entry| {
            let s = entry.to_string();
            !s.contains(HOOK_MARKER)
        });
    }

    match write_json(&config_path, &settings) {
        Ok(()) => InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::Installed,
        },
        Err(e) => InstallResult {
            agent: agent_name,
            config_path,
            status: InstallStatus::Failed(e),
        },
    }
}

fn write_json(path: &std::path::Path, value: &serde_json::Value) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}
