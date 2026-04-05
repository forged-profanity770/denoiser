use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

static KUBECTL_EVENT_NOISE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        ^\d+[smh]\s+(?:Normal|Warning)\s+(?:
            Scheduled   |
            Pulling     |
            Pulled      |
            Created     |
            Started     |
            ScalingReplicaSet |
            SuccessfulCreate
        )",
    )
    .expect("kubectl event regex valid")
});

static KUBECTL_VERBOSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^I\d{4}\s+\d+:\d+:\d+\.\d+\s+").expect("kubectl verbose log regex valid")
});

static KUBECTL_API_CALL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:GET|POST|PUT|PATCH|DELETE)\s+https?://").expect("kubectl api call regex valid")
});

/// kubectl/k8s-specific noise filter.
///
/// Strips: routine scheduling events, verbose API logging (I0405 ...),
/// raw API call traces.
///
/// Preserves: pod status, errors, warnings with meaningful content,
/// resource output (get, describe), logs output.
pub struct KubectlFilter;

impl Filter for KubectlFilter {
    fn name(&self) -> &'static str {
        "kubectl"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return FilterResult::Keep;
        }

        // Verbose klog output (I0405 12:34:56.789 ...)
        if KUBECTL_VERBOSE.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Raw API call traces
        if KUBECTL_API_CALL.is_match(trimmed) {
            return FilterResult::Drop;
        }

        FilterResult::Keep
    }

    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(lines.len());
        let mut event_count: usize = 0;

        for line in lines {
            let trimmed = line.trim();

            if KUBECTL_EVENT_NOISE.is_match(trimmed) {
                event_count += 1;
                continue;
            }

            if event_count > 0 {
                result.push(format!("[{event_count} routine k8s events collapsed]"));
                event_count = 0;
            }
            result.push(line.clone());
        }

        if event_count > 0 {
            result.push(format!("[{event_count} routine k8s events collapsed]"));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_verbose_logs() {
        let filter = KubectlFilter;
        assert_eq!(
            filter.filter_line("I0405 12:34:56.789012 12345 request.go:1073] Response"),
            FilterResult::Drop
        );
    }

    #[test]
    fn drops_api_traces() {
        let filter = KubectlFilter;
        assert_eq!(
            filter.filter_line("GET https://127.0.0.1:6443/api/v1/pods"),
            FilterResult::Drop
        );
    }

    #[test]
    fn keeps_pod_status() {
        let filter = KubectlFilter;
        assert_eq!(
            filter.filter_line("nginx-7d4b8c8f9-x2k4l   1/1     Running   0          5m"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_errors() {
        let filter = KubectlFilter;
        assert_eq!(
            filter.filter_line("Error from server (NotFound): pods \"foo\" not found"),
            FilterResult::Keep
        );
    }

    #[test]
    fn block_collapses_events() {
        let filter = KubectlFilter;
        let lines = vec![
            "LAST SEEN   TYPE     REASON    OBJECT".to_string(),
            "2m          Normal   Scheduled pod/nginx-abc".to_string(),
            "2m          Normal   Pulling   pod/nginx-abc".to_string(),
            "1m          Normal   Pulled    pod/nginx-abc".to_string(),
            "1m          Normal   Created   pod/nginx-abc".to_string(),
            "1m          Normal   Started   pod/nginx-abc".to_string(),
            "pod/nginx-abc is ready".to_string(),
        ];
        let result = filter.filter_block(&lines);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "LAST SEEN   TYPE     REASON    OBJECT");
        assert!(result[1].contains("5 routine k8s events"));
        assert_eq!(result[2], "pod/nginx-abc is ready");
    }
}
