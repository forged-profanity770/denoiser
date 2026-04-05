use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

static NPM_WARN_DEPRECATED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^npm\s+warn\s+deprecated\s+").expect("npm warn deprecated regex valid")
});

static NPM_TIMING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^npm\s+timing\s+").expect("npm timing regex valid"));

static NPM_HTTP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^npm\s+http\s+(?:fetch\s+)?(?:GET|POST|PUT)\s+").expect("npm http regex valid")
});

static NPM_NOTICE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^npm\s+notice\s*$").expect("npm empty notice regex valid"));

static NPM_WARN_PEER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^npm\s+warn\s+(?:ERESOLVE|peer\s+dep|overriding\s+peer)")
        .expect("npm warn peer regex valid")
});

/// npm/yarn/pnpm-specific noise filter.
///
/// Strips: deprecation warnings (keeps count), timing logs, HTTP fetch logs,
/// empty notice lines, peer dependency resolution noise.
///
/// Preserves: actual errors, audit vulnerabilities, the "added N packages" summary,
/// postinstall script output, build errors.
pub struct NpmFilter;

impl Filter for NpmFilter {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return FilterResult::Keep;
        }

        // npm timing logs (verbose debug output)
        if NPM_TIMING.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // npm HTTP fetch logs
        if NPM_HTTP.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Empty "npm notice" lines
        if NPM_NOTICE.is_match(trimmed) {
            return FilterResult::Drop;
        }

        FilterResult::Keep
    }

    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(lines.len());
        let mut deprecated_count: usize = 0;
        let mut peer_warn_count: usize = 0;

        for line in lines {
            let trimmed = line.trim();

            if NPM_WARN_DEPRECATED.is_match(trimmed) {
                deprecated_count += 1;
                continue;
            }

            if NPM_WARN_PEER.is_match(trimmed) {
                peer_warn_count += 1;
                continue;
            }

            result.push(line.clone());
        }

        // Emit summaries for collapsed warnings
        if deprecated_count > 0 {
            result.push(format!(
                "[{deprecated_count} npm deprecation warning{} collapsed]",
                if deprecated_count == 1 { "" } else { "s" }
            ));
        }

        if peer_warn_count > 0 {
            result.push(format!(
                "[{peer_warn_count} npm peer dependency warning{} collapsed]",
                if peer_warn_count == 1 { "" } else { "s" }
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_timing_logs() {
        let filter = NpmFilter;
        assert_eq!(
            filter.filter_line("npm timing idealTree Completed in 234ms"),
            FilterResult::Drop
        );
    }

    #[test]
    fn drops_http_logs() {
        let filter = NpmFilter;
        assert_eq!(
            filter.filter_line("npm http fetch GET https://registry.npmjs.org/react"),
            FilterResult::Drop
        );
    }

    #[test]
    fn keeps_errors() {
        let filter = NpmFilter;
        assert_eq!(
            filter.filter_line("npm error code ERESOLVE"),
            FilterResult::Keep
        );
    }

    #[test]
    fn block_collapses_deprecation_warnings() {
        let filter = NpmFilter;
        let lines = vec![
            "npm warn deprecated glob@7.2.3: use v9".to_string(),
            "npm warn deprecated inflight@1.0.6: no longer maintained".to_string(),
            "npm warn deprecated rimraf@3.0.2: use v4".to_string(),
            "added 150 packages in 4s".to_string(),
        ];
        let result = filter.filter_block(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "added 150 packages in 4s");
        assert!(result[1].contains("3 npm deprecation warnings collapsed"));
    }

    #[test]
    fn keeps_added_packages_summary() {
        let filter = NpmFilter;
        assert_eq!(
            filter.filter_line("added 150 packages, and audited 151 packages in 4s"),
            FilterResult::Keep
        );
    }
}
