use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

// Matches common progress bar patterns:
// [=====>    ] 50%
// ████████░░░░ 75%
// 50% |=====     |
// (3/10) Installing...
// Downloading: 45.2 MB / 100.0 MB
static PROGRESS_PERCENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|\s)\d{1,3}(?:\.\d+)?%(?:\s|$|\|)").expect("progress percent regex valid")
});

static PROGRESS_BAR_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[=\-#>█▓▒░╸╺┃│\|]{5,}").expect("progress bar chars regex valid"));

// Only Unicode spinners -- ASCII |/-\ are too common in code output
// and would cause false positives on compiler errors, paths, etc.
static SPINNER_CHARS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏⣾⣽⣻⢿⡿⣟⣯⣷◐◓◑◒]\s").expect("spinner regex valid"));

/// Detects and collapses progress bars, spinners, and download indicators.
/// These are purely visual feedback -- an LLM needs only the final state.
pub struct ProgressFilter;

impl Filter for ProgressFilter {
    fn name(&self) -> &'static str {
        "progress"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        if is_progress_line(line) {
            FilterResult::Drop
        } else {
            FilterResult::Keep
        }
    }

    /// Collapse consecutive progress lines into a single summary.
    /// e.g., 50 lines of "Downloading... X%" become "[progress: Downloading]"
    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(lines.len());
        let mut in_progress_run = false;
        let mut progress_context: Option<String> = None;

        for line in lines {
            if is_progress_line(line) {
                if !in_progress_run {
                    in_progress_run = true;
                    progress_context = extract_progress_context(line);
                }
            } else {
                if in_progress_run {
                    // Emit a single summary line for the collapsed progress block
                    let ctx = progress_context.take().unwrap_or_default();
                    if ctx.is_empty() {
                        result.push("[progress collapsed]".to_string());
                    } else {
                        result.push(format!("[progress: {ctx}]"));
                    }
                    in_progress_run = false;
                }
                result.push(line.clone());
            }
        }

        // Handle trailing progress run
        if in_progress_run {
            let ctx = progress_context.take().unwrap_or_default();
            if ctx.is_empty() {
                result.push("[progress collapsed]".to_string());
            } else {
                result.push(format!("[progress: {ctx}]"));
            }
        }

        result
    }
}

fn is_progress_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Spinner at start of line
    if SPINNER_CHARS.is_match(trimmed) {
        return true;
    }

    // Has both percentage and bar characters (high confidence)
    let has_percent = PROGRESS_PERCENT.is_match(trimmed);
    let has_bar = PROGRESS_BAR_CHARS.is_match(trimmed);
    if has_percent && has_bar {
        return true;
    }

    // Pure progress bar line (just bar characters and whitespace)
    if has_bar && trimmed.len() < 120 {
        let non_bar = trimmed
            .chars()
            .filter(|c| {
                !matches!(
                    c,
                    '=' | '-'
                        | '#'
                        | '>'
                        | '█'
                        | '▓'
                        | '▒'
                        | '░'
                        | '╸'
                        | '╺'
                        | '┃'
                        | '│'
                        | '|'
                        | ' '
                        | '['
                        | ']'
                )
            })
            .count();
        // If <30% of chars are non-bar, it's a progress bar
        if non_bar < trimmed.len() / 3 {
            return true;
        }
    }

    false
}

fn extract_progress_context(line: &str) -> Option<String> {
    let trimmed = line.trim();
    // Try to extract the action word before the progress indicator
    // e.g., "Downloading packages... 50%" -> "Downloading packages"
    let without_progress = PROGRESS_PERCENT.replace(trimmed, "").trim().to_string();
    let without_bar = PROGRESS_BAR_CHARS
        .replace_all(&without_progress, "")
        .trim()
        .to_string();
    let cleaned = without_bar
        .trim_matches(|c: char| c == '[' || c == ']' || c == '|' || c == ' ')
        .trim_end_matches("...")
        .trim()
        .to_string();

    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_percent_with_bar() {
        assert!(is_progress_line("[=====>    ] 50%"));
        assert!(is_progress_line("████████░░░░ 75% "));
    }

    #[test]
    fn detects_spinner() {
        assert!(is_progress_line("⠋ Installing dependencies"));
        assert!(is_progress_line("⣾ Building..."));
        // ASCII spinners (|/-\) are NOT detected to avoid false positives
        assert!(!is_progress_line("/ Building..."));
        assert!(!is_progress_line("| something"));
    }

    #[test]
    fn ignores_normal_text() {
        assert!(!is_progress_line("error: something failed"));
        assert!(!is_progress_line("warning: unused variable"));
        assert!(!is_progress_line("src/main.rs:5:10"));
    }

    #[test]
    fn ignores_code_with_pipes() {
        // Code that happens to contain | chars should NOT be detected
        assert!(!is_progress_line("match x { 1 => a, 2 => b }"));
    }

    #[test]
    fn block_filter_collapses_progress_run() {
        let filter = ProgressFilter;
        let lines = vec![
            "Starting download".to_string(),
            "[=====>     ] 40%".to_string(),
            "[========>  ] 80%".to_string(),
            "[==========] 100%".to_string(),
            "Download complete".to_string(),
        ];
        let result = filter.filter_block(&lines);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Starting download");
        assert!(result[1].starts_with("[progress"));
        assert_eq!(result[2], "Download complete");
    }
}
