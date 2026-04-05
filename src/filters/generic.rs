use super::{Filter, FilterResult};

/// Generic fallback filter for unknown commands.
/// Only strips patterns that are UNIVERSALLY noise regardless of context:
/// - Lines that are purely whitespace
/// - Lines that are purely box-drawing characters (decorative borders)
///
/// This filter is maximally conservative. When in doubt, keep everything.
pub struct GenericFilter;

impl Filter for GenericFilter {
    fn name(&self) -> &'static str {
        "generic"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let trimmed = line.trim();

        // Keep empty lines (they're structural)
        if trimmed.is_empty() {
            return FilterResult::Keep;
        }

        // Drop lines that are purely decorative box-drawing
        if is_decorative_border(trimmed) {
            return FilterResult::Drop;
        }

        FilterResult::Keep
    }
}

/// Check if a line is purely decorative (box-drawing chars, dashes, equals).
/// Must be at least 4 chars to avoid false positives on short dashes.
fn is_decorative_border(line: &str) -> bool {
    if line.len() < 10 {
        return false;
    }

    let total = line.chars().count();
    let border_chars = line
        .chars()
        .filter(|c| {
            matches!(
                c,
                '─' | '━'
                    | '│'
                    | '┃'
                    | '┌'
                    | '┐'
                    | '└'
                    | '┘'
                    | '├'
                    | '┤'
                    | '┬'
                    | '┴'
                    | '┼'
                    | '═'
                    | '║'
                    | '╔'
                    | '╗'
                    | '╚'
                    | '╝'
                    | '╠'
                    | '╣'
                    | '╦'
                    | '╩'
                    | '╬'
                    | '-'
                    | '='
                    | '+'
                    | '|'
                    | '*'
                    | ' '
            )
        })
        .count();

    // 95%+ border characters = decorative
    border_chars * 100 / total >= 95
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_dashed_borders() {
        let filter = GenericFilter;
        assert_eq!(
            filter.filter_line("----------------------------------------"),
            FilterResult::Drop
        );
        assert_eq!(
            filter.filter_line("========================================"),
            FilterResult::Drop
        );
    }

    #[test]
    fn drops_box_drawing() {
        let filter = GenericFilter;
        assert_eq!(
            filter.filter_line("┌──────────────────────────────────────┐"),
            FilterResult::Drop
        );
        assert_eq!(
            filter.filter_line("└──────────────────────────────────────┘"),
            FilterResult::Drop
        );
    }

    #[test]
    fn keeps_short_dashes() {
        let filter = GenericFilter;
        // Short dashes could be flags or list items
        assert_eq!(filter.filter_line("---"), FilterResult::Keep);
        assert_eq!(filter.filter_line("--help"), FilterResult::Keep);
    }

    #[test]
    fn keeps_text_with_dashes() {
        let filter = GenericFilter;
        assert_eq!(
            filter.filter_line("error: something -- failed here"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_normal_text() {
        let filter = GenericFilter;
        assert_eq!(filter.filter_line("hello world"), FilterResult::Keep);
    }
}
