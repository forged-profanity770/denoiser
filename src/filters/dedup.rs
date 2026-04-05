use super::{Filter, FilterResult};

const DEDUP_THRESHOLD: usize = 3;

/// Collapses consecutive duplicate lines.
/// e.g., 50 identical "npm warn deprecated" lines become one + "[repeated 49x]"
///
/// Uses exact-match deduplication only (zero false positives).
/// Near-duplicate detection is deliberately excluded from v1
/// to avoid collapsing lines that look similar but carry different info.
pub struct DedupFilter;

impl DedupFilter {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DedupFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl Filter for DedupFilter {
    fn name(&self) -> &'static str {
        "dedup"
    }

    fn filter_line(&self, _line: &str) -> FilterResult {
        // Dedup operates at block level, not line level
        FilterResult::Keep
    }

    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        collapse_duplicates(lines)
    }
}

fn collapse_duplicates(lines: &[String]) -> Vec<String> {
    let mut result = Vec::with_capacity(lines.len());
    let mut run_count: usize = 0;
    let mut run_line: Option<&str> = None;

    for line in lines {
        let trimmed = line.trim();
        if let Some(prev) = run_line {
            if trimmed == prev {
                run_count += 1;
                continue;
            }
            flush_run(&mut result, prev, run_count);
        }
        run_line = Some(trimmed);
        run_count = 1;
    }

    if let Some(prev) = run_line {
        flush_run(&mut result, prev, run_count);
    }

    result
}

fn flush_run(result: &mut Vec<String>, line: &str, count: usize) {
    if count >= DEDUP_THRESHOLD {
        result.push(line.to_string());
        result.push(format!("[repeated {count}x]"));
    } else {
        for _ in 0..count {
            result.push(line.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_exact_duplicates() {
        let lines: Vec<String> = (0..10)
            .map(|_| "npm warn deprecated glob@7.2.3".to_string())
            .collect();
        let result = collapse_duplicates(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "npm warn deprecated glob@7.2.3");
        assert_eq!(result[1], "[repeated 10x]");
    }

    #[test]
    fn preserves_unique_lines() {
        let lines = vec![
            "line one".to_string(),
            "line two".to_string(),
            "line three".to_string(),
        ];
        let result = collapse_duplicates(&lines);
        assert_eq!(result, lines);
    }

    #[test]
    fn below_threshold_not_collapsed() {
        let lines = vec!["same".to_string(), "same".to_string()];
        let result = collapse_duplicates(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "same");
        assert_eq!(result[1], "same");
    }

    #[test]
    fn mixed_runs() {
        let lines = vec![
            "start".to_string(),
            "warn: x".to_string(),
            "warn: x".to_string(),
            "warn: x".to_string(),
            "warn: x".to_string(),
            "end".to_string(),
        ];
        let result = collapse_duplicates(&lines);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "start");
        assert_eq!(result[1], "warn: x");
        assert_eq!(result[2], "[repeated 4x]");
        assert_eq!(result[3], "end");
    }
}
