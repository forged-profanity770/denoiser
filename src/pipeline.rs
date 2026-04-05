use crate::filters::{Filter, FilterResult};

/// Pipeline chains filters and applies them sequentially.
/// Core invariant: if any filter is uncertain, the original line passes through.
#[derive(Default)]
pub struct Pipeline {
    filters: Vec<Box<dyn Filter>>,
}

impl Pipeline {
    #[must_use]
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add_filter(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    /// Process a full output string through all filters.
    /// Returns the filtered output and the token savings estimate.
    #[must_use]
    pub fn process(&self, input: &str) -> PipelineResult {
        let mut lines: Vec<String> = input.lines().map(String::from).collect();
        let original_tokens = estimate_tokens(input);

        for filter in &self.filters {
            let mut new_lines = Vec::with_capacity(lines.len());
            for line in &lines {
                match filter.filter_line(line) {
                    FilterResult::Replace(replacement) => new_lines.push(replacement),
                    FilterResult::Drop => {}
                    FilterResult::Keep | FilterResult::Uncertain => {
                        new_lines.push(line.clone());
                    }
                }
            }
            lines = new_lines;
        }

        // Second pass: block-level filters
        for filter in &self.filters {
            lines = filter.filter_block(&lines);
        }

        let output = lines.join("\n");
        let filtered_tokens = estimate_tokens(&output);

        PipelineResult {
            output,
            original_tokens,
            filtered_tokens,
            savings: original_tokens.saturating_sub(filtered_tokens),
        }
    }
}

/// Estimate token count. Uses `char_count/4` as a rough heuristic,
/// consistent with industry standard approximation for English text.
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub output: String,
    pub original_tokens: usize,
    pub filtered_tokens: usize,
    pub savings: usize,
}

impl PipelineResult {
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn savings_percent(&self) -> f64 {
        if self.original_tokens == 0 {
            return 0.0;
        }
        (self.savings as f64 / self.original_tokens as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{Filter, FilterResult};

    struct DropEmptyLines;
    impl Filter for DropEmptyLines {
        fn name(&self) -> &'static str {
            "drop-empty"
        }
        fn filter_line(&self, line: &str) -> FilterResult {
            if line.trim().is_empty() {
                FilterResult::Drop
            } else {
                FilterResult::Keep
            }
        }
    }

    #[test]
    fn pipeline_drops_empty_lines() {
        let mut pipeline = Pipeline::new();
        pipeline.add_filter(Box::new(DropEmptyLines));
        let result = pipeline.process("hello\n\nworld\n\n");
        assert_eq!(result.output, "hello\nworld");
        assert!(result.savings > 0);
    }

    #[test]
    fn empty_input_returns_zero_savings() {
        let pipeline = Pipeline::new();
        let result = pipeline.process("");
        assert_eq!(result.savings, 0);
        assert!((result.savings_percent() - 0.0).abs() < f64::EPSILON);
    }
}
