pub mod ansi;
pub mod dedup;
pub mod generic;
pub mod git;
pub mod npm;
pub mod progress;

/// Result of filtering a single line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterResult {
    /// Keep the line as-is.
    Keep,
    /// Replace the line with a new string.
    Replace(String),
    /// Drop the line entirely (confirmed noise).
    Drop,
    /// Filter is uncertain -- line passes through unchanged.
    /// This is the zero-false-positive guarantee.
    Uncertain,
}

/// Trait that all filters implement.
pub trait Filter: Send + Sync {
    /// Human-readable name for logging and stats.
    fn name(&self) -> &'static str;

    /// Filter a single line. Return `Uncertain` when unsure.
    fn filter_line(&self, line: &str) -> FilterResult;

    /// Filter at block level (multi-line patterns).
    /// Default: return lines unchanged.
    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        lines.to_vec()
    }
}

/// Detect which command is being run from the command string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    Git,
    Npm,
    Cargo,
    Docker,
    Kubectl,
    Unknown,
}

impl CommandKind {
    #[must_use]
    pub fn detect(command: &str) -> Self {
        let cmd = command.split_whitespace().next().unwrap_or("");
        // Strip path prefix (e.g., /usr/bin/git -> git)
        let base = cmd.rsplit('/').next().unwrap_or(cmd);
        match base {
            "git" => Self::Git,
            "npm" | "npx" | "yarn" | "pnpm" | "bun" => Self::Npm,
            "cargo" | "rustc" | "rustup" => Self::Cargo,
            "docker" | "docker-compose" | "podman" => Self::Docker,
            "kubectl" | "k9s" | "helm" => Self::Kubectl,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_git() {
        assert_eq!(CommandKind::detect("git status"), CommandKind::Git);
        assert_eq!(CommandKind::detect("/usr/bin/git log"), CommandKind::Git);
    }

    #[test]
    fn detect_npm_variants() {
        assert_eq!(CommandKind::detect("npm install"), CommandKind::Npm);
        assert_eq!(CommandKind::detect("pnpm dev"), CommandKind::Npm);
        assert_eq!(CommandKind::detect("bun run test"), CommandKind::Npm);
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(CommandKind::detect("ls -la"), CommandKind::Unknown);
        assert_eq!(CommandKind::detect(""), CommandKind::Unknown);
    }
}
