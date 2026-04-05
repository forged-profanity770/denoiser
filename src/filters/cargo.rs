use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

static CARGO_COMPILING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*Compiling\s+\S+\s+v").expect("cargo compiling regex valid"));

static CARGO_DOWNLOADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:Downloading|Downloaded)\s+").expect("cargo downloading regex valid")
});

static CARGO_FRESH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*Fresh\s+\S+\s+v").expect("cargo fresh regex valid"));

static CARGO_CHECKING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*Checking\s+\S+\s+v").expect("cargo checking regex valid"));

static CARGO_LOCKING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:Locking|Updating|Adding)\s+").expect("cargo locking regex valid")
});

/// Cargo/Rust-specific noise filter.
///
/// Strips: Compiling/Checking/Fresh/Downloading per-crate lines,
/// Locking/Updating dependency resolution, download progress.
///
/// Preserves: errors, warnings, test results, Finished summary,
/// benchmark output, doc output, any diagnostic.
pub struct CargoFilter;

impl Filter for CargoFilter {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return FilterResult::Keep;
        }

        // Per-crate compilation lines (Compiling foo v1.2.3)
        if CARGO_COMPILING.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Download lines
        if CARGO_DOWNLOADING.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Fresh (already compiled) lines
        if CARGO_FRESH.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Checking lines
        if CARGO_CHECKING.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Locking/Updating dependency resolution
        if CARGO_LOCKING.is_match(trimmed) {
            return FilterResult::Drop;
        }

        FilterResult::Keep
    }

    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(lines.len());
        let mut compile_count: usize = 0;
        let mut download_count: usize = 0;

        for line in lines {
            let trimmed = line.trim();

            if CARGO_COMPILING.is_match(trimmed) || CARGO_CHECKING.is_match(trimmed) {
                compile_count += 1;
                continue;
            }

            if CARGO_DOWNLOADING.is_match(trimmed) {
                download_count += 1;
                continue;
            }

            if CARGO_FRESH.is_match(trimmed) || CARGO_LOCKING.is_match(trimmed) {
                continue;
            }

            // Emit summaries before non-noise lines
            emit_summaries(&mut result, &mut compile_count, &mut download_count);
            result.push(line.clone());
        }

        emit_summaries(&mut result, &mut compile_count, &mut download_count);
        result
    }
}

fn emit_summaries(result: &mut Vec<String>, compile_count: &mut usize, download_count: &mut usize) {
    if *compile_count > 0 {
        result.push(format!("[compiled {compile_count} crates]"));
        *compile_count = 0;
    }
    if *download_count > 0 {
        result.push(format!("[downloaded {download_count} crates]"));
        *download_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_compiling_lines() {
        let filter = CargoFilter;
        assert_eq!(
            filter.filter_line("   Compiling serde v1.0.228"),
            FilterResult::Drop
        );
        assert_eq!(
            filter.filter_line("   Compiling tokio v1.51.0"),
            FilterResult::Drop
        );
    }

    #[test]
    fn drops_fresh_lines() {
        let filter = CargoFilter;
        assert_eq!(
            filter.filter_line("       Fresh regex v1.12.3"),
            FilterResult::Drop
        );
    }

    #[test]
    fn keeps_errors_and_warnings() {
        let filter = CargoFilter;
        assert_eq!(
            filter.filter_line("error[E0308]: mismatched types"),
            FilterResult::Keep
        );
        assert_eq!(
            filter.filter_line("warning: unused variable `x`"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_finished_line() {
        let filter = CargoFilter;
        assert_eq!(
            filter.filter_line("    Finished `dev` profile in 2.34s"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_test_results() {
        let filter = CargoFilter;
        assert_eq!(
            filter.filter_line("test result: ok. 37 passed; 0 failed"),
            FilterResult::Keep
        );
        assert_eq!(filter.filter_line("running 37 tests"), FilterResult::Keep);
    }

    #[test]
    fn block_collapses_compilation() {
        let filter = CargoFilter;
        let lines = vec![
            "   Compiling serde v1.0.228".to_string(),
            "   Compiling tokio v1.51.0".to_string(),
            "   Compiling regex v1.12.3".to_string(),
            "   Compiling cli-denoiser v0.1.0".to_string(),
            "    Finished `dev` profile in 2.34s".to_string(),
        ];
        let result = filter.filter_block(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "[compiled 4 crates]");
        assert!(result[1].contains("Finished"));
    }
}
