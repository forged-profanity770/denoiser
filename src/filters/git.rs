use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

static GIT_PUSH_NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        ^(?:
            Enumerating\sobjects     |
            Counting\sobjects        |
            Compressing\sobjects     |
            Delta\s(?:resolution|compression) |
            Writing\sobjects         |
            Total\s\d+               |
            remote:\s*$              |
            remote:\sCompressing     |
            remote:\sCounting        |
            remote:\sResolving       |
            remote:\sTotal           |
            \s*\(delta\s\d+\)
        )",
    )
    .expect("git push noise regex valid")
});

static GIT_FETCH_NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        ^(?:
            remote:\sEnumerating   |
            remote:\sCounting      |
            remote:\sCompressing   |
            Receiving\sobjects     |
            Resolving\sdeltas      |
            Unpacking\sobjects     |
            From\s                 |
            POST\sgit-upload-pack
        )",
    )
    .expect("git fetch noise regex valid")
});

static GIT_CLONE_PROGRESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:Cloning into|Receiving|Resolving|Updating files).*\d+%")
        .expect("git clone progress regex valid")
});

/// Git-specific noise filter.
/// Strips transfer stats, pack compression, and delta resolution lines
/// that carry zero information for an LLM.
///
/// Preserves: branch names, commit hashes, conflict markers,
/// error messages, diff output, status output.
pub struct GitFilter;

impl Filter for GitFilter {
    fn name(&self) -> &'static str {
        "git"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let trimmed = line.trim();

        // Never filter empty lines (context-dependent)
        if trimmed.is_empty() {
            return FilterResult::Keep;
        }

        // git push/pull transfer noise
        if GIT_PUSH_NOISE.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // git fetch/clone transfer noise
        if GIT_FETCH_NOISE.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // git clone progress with percentages
        if GIT_CLONE_PROGRESS.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // "remote:" lines that are just whitespace
        if trimmed == "remote:" {
            return FilterResult::Drop;
        }

        FilterResult::Keep
    }

    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(lines.len());
        let mut dropped_transfer = false;

        for line in lines {
            let trimmed = line.trim();
            let is_noise = GIT_PUSH_NOISE.is_match(trimmed)
                || GIT_FETCH_NOISE.is_match(trimmed)
                || GIT_CLONE_PROGRESS.is_match(trimmed)
                || trimmed == "remote:";

            if is_noise {
                if !dropped_transfer {
                    dropped_transfer = true;
                }
            } else {
                if dropped_transfer {
                    result.push("[git transfer stats collapsed]".to_string());
                    dropped_transfer = false;
                }
                result.push(line.clone());
            }
        }

        if dropped_transfer {
            result.push("[git transfer stats collapsed]".to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_push_stats() {
        let filter = GitFilter;
        assert_eq!(
            filter.filter_line("Enumerating objects: 15, done."),
            FilterResult::Drop
        );
        assert_eq!(
            filter.filter_line("Counting objects: 100% (15/15), done."),
            FilterResult::Drop
        );
        assert_eq!(
            filter.filter_line("Writing objects: 100% (8/8), 2.51 KiB | 2.51 MiB/s, done."),
            FilterResult::Drop
        );
    }

    #[test]
    fn keeps_branch_info() {
        let filter = GitFilter;
        assert_eq!(
            filter.filter_line("   abc1234..def5678  main -> main"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_error_messages() {
        let filter = GitFilter;
        assert_eq!(
            filter.filter_line("error: failed to push some refs"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_status_output() {
        let filter = GitFilter;
        assert_eq!(filter.filter_line("M  src/main.rs"), FilterResult::Keep);
        assert_eq!(filter.filter_line("?? new_file.txt"), FilterResult::Keep);
    }

    #[test]
    fn block_collapses_transfer() {
        let filter = GitFilter;
        let lines = vec![
            "Enumerating objects: 15, done.".to_string(),
            "Counting objects: 100% (15/15), done.".to_string(),
            "Delta compression using up to 8 threads".to_string(),
            "Compressing objects: 100% (8/8), done.".to_string(),
            "Writing objects: 100% (8/8), 2.51 KiB, done.".to_string(),
            "   abc1234..def5678  main -> main".to_string(),
        ];
        let result = filter.filter_block(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "[git transfer stats collapsed]");
        assert!(result[1].contains("main -> main"));
    }
}
