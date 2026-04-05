use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Matches all ANSI escape sequences:
    // CSI sequences (colors, cursor movement, etc.)
    // OSC sequences (terminal title, hyperlinks)
    // Simple escape codes (bold, reset, etc.)
    Regex::new(concat!(
        r"\x1b\[[0-9;]*[A-Za-z]", // CSI: \e[...m, \e[...H, etc.
        r"|\x1b\][^\x07]*\x07",   // OSC: \e]...\a
        r"|\x1b\][^\x1b]*\x1b\\", // OSC: \e]...\e\
        r"|\x1b[()][AB012]",      // Character set selection
        r"|\x1b[=>Nno|{}~]",      // Various mode switches
        r"|\x0f|\x0e",            // SI/SO (shift in/out)
        r"|\r",                   // Carriage return (progress bar artifact)
    ))
    .expect("ANSI regex is valid")
});

/// Strips all ANSI escape codes from terminal output.
/// This is a pure signal-preserving operation -- ANSI codes carry
/// zero semantic information for an LLM.
pub struct AnsiFilter;

impl Filter for AnsiFilter {
    fn name(&self) -> &'static str {
        "ansi"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let cleaned = strip_ansi(line);
        if cleaned == line {
            FilterResult::Keep
        } else if cleaned.trim().is_empty() {
            // Line was purely ANSI codes (e.g., color reset)
            FilterResult::Drop
        } else {
            FilterResult::Replace(cleaned)
        }
    }
}

/// Strip all ANSI escape sequences from a string.
#[must_use]
pub fn strip_ansi(input: &str) -> String {
    ANSI_REGEX.replace_all(input, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_color_codes() {
        let input = "\x1b[32m✓\x1b[0m test passed";
        assert_eq!(strip_ansi(input), "✓ test passed");
    }

    #[test]
    fn strips_bold_and_underline() {
        let input = "\x1b[1m\x1b[4mBold Underline\x1b[0m";
        assert_eq!(strip_ansi(input), "Bold Underline");
    }

    #[test]
    fn preserves_clean_text() {
        let input = "no ansi here";
        assert_eq!(strip_ansi(input), "no ansi here");
    }

    #[test]
    fn strips_cursor_movement() {
        let input = "\x1b[2K\x1b[1GDownloading...";
        assert_eq!(strip_ansi(input), "Downloading...");
    }

    #[test]
    fn drops_pure_ansi_lines() {
        let filter = AnsiFilter;
        let result = filter.filter_line("\x1b[0m\x1b[K");
        assert_eq!(result, FilterResult::Drop);
    }

    #[test]
    fn filter_replaces_mixed_lines() {
        let filter = AnsiFilter;
        let result = filter.filter_line("\x1b[32mhello\x1b[0m");
        assert_eq!(result, FilterResult::Replace("hello".to_string()));
    }

    #[test]
    fn filter_keeps_clean_lines() {
        let filter = AnsiFilter;
        assert_eq!(filter.filter_line("clean text"), FilterResult::Keep);
    }

    #[test]
    fn strips_carriage_returns() {
        let input = "Progress: 50%\rProgress: 100%";
        assert_eq!(strip_ansi(input), "Progress: 50%Progress: 100%");
    }
}
