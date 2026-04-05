use regex::Regex;
use std::sync::LazyLock;

use super::{Filter, FilterResult};

static DOCKER_LAYER_CACHE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:--->\s+[a-f0-9]{12}|---> Using cache|Removing intermediate container)")
        .expect("docker layer regex valid")
});

static DOCKER_PULL_PROGRESS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[a-f0-9]{12}:\s+(?:Pulling|Waiting|Downloading|Extracting|Verifying|Pull complete)",
    )
    .expect("docker pull regex valid")
});

static DOCKER_DIGEST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:Digest:\s+sha256:|Status:\s+Downloaded newer image)")
        .expect("docker digest regex valid")
});

/// Docker-specific noise filter.
///
/// Strips: layer cache hits, intermediate container IDs, pull progress,
/// image digest lines.
///
/// Preserves: build step commands (Step N/M : RUN ...), errors,
/// container output, final image ID, COPY/ADD context.
pub struct DockerFilter;

impl Filter for DockerFilter {
    fn name(&self) -> &'static str {
        "docker"
    }

    fn filter_line(&self, line: &str) -> FilterResult {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return FilterResult::Keep;
        }

        // Layer cache and intermediate container noise
        if DOCKER_LAYER_CACHE.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Pull progress per-layer
        if DOCKER_PULL_PROGRESS.is_match(trimmed) {
            return FilterResult::Drop;
        }

        // Digest/status lines
        if DOCKER_DIGEST.is_match(trimmed) {
            return FilterResult::Drop;
        }

        FilterResult::Keep
    }

    fn filter_block(&self, lines: &[String]) -> Vec<String> {
        let mut result = Vec::with_capacity(lines.len());
        let mut pull_count: usize = 0;
        let mut cache_count: usize = 0;

        for line in lines {
            let trimmed = line.trim();

            if DOCKER_PULL_PROGRESS.is_match(trimmed) || DOCKER_DIGEST.is_match(trimmed) {
                pull_count += 1;
                continue;
            }

            if DOCKER_LAYER_CACHE.is_match(trimmed) {
                cache_count += 1;
                continue;
            }

            emit_summaries(&mut result, &mut pull_count, &mut cache_count);
            result.push(line.clone());
        }

        emit_summaries(&mut result, &mut pull_count, &mut cache_count);
        result
    }
}

fn emit_summaries(result: &mut Vec<String>, pull_count: &mut usize, cache_count: &mut usize) {
    if *pull_count > 0 {
        result.push(format!("[pulled {pull_count} layers]"));
        *pull_count = 0;
    }
    if *cache_count > 0 {
        result.push(format!("[{cache_count} cached layers]"));
        *cache_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drops_layer_cache() {
        let filter = DockerFilter;
        assert_eq!(filter.filter_line("---> Using cache"), FilterResult::Drop);
        assert_eq!(filter.filter_line("---> a1b2c3d4e5f6"), FilterResult::Drop);
    }

    #[test]
    fn drops_pull_progress() {
        let filter = DockerFilter;
        assert_eq!(
            filter.filter_line("a1b2c3d4e5f6: Downloading  45.2MB/100.0MB"),
            FilterResult::Drop
        );
        assert_eq!(
            filter.filter_line("a1b2c3d4e5f6: Pull complete"),
            FilterResult::Drop
        );
    }

    #[test]
    fn keeps_build_steps() {
        let filter = DockerFilter;
        assert_eq!(
            filter.filter_line("Step 3/10 : RUN apt-get update"),
            FilterResult::Keep
        );
    }

    #[test]
    fn keeps_errors() {
        let filter = DockerFilter;
        assert_eq!(
            filter.filter_line("ERROR: failed to solve: process did not complete"),
            FilterResult::Keep
        );
    }

    #[test]
    fn block_collapses_pull() {
        let filter = DockerFilter;
        let lines = vec![
            "Using default tag: latest".to_string(),
            "a1b2c3d4e5f6: Pulling fs layer".to_string(),
            "b2c3d4e5f6a1: Pulling fs layer".to_string(),
            "a1b2c3d4e5f6: Downloading  45MB/100MB".to_string(),
            "a1b2c3d4e5f6: Pull complete".to_string(),
            "b2c3d4e5f6a1: Pull complete".to_string(),
            "Digest: sha256:abc123".to_string(),
            "Status: Downloaded newer image for node:20".to_string(),
        ];
        let result = filter.filter_block(&lines);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "Using default tag: latest");
        assert!(result[1].contains("pulled"));
    }
}
