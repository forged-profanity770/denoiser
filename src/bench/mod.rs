mod corpus;

use std::time::Instant;

use crate::filters::ansi::AnsiFilter;
use crate::filters::cargo::CargoFilter;
use crate::filters::dedup::DedupFilter;
use crate::filters::docker::DockerFilter;
use crate::filters::generic::GenericFilter;
use crate::filters::git::GitFilter;
use crate::filters::kubectl::KubectlFilter;
use crate::filters::npm::NpmFilter;
use crate::filters::progress::ProgressFilter;
use crate::filters::{CommandKind, Filter};
use crate::pipeline::Pipeline;

/// Full benchmark results.
#[derive(Debug, serde::Serialize)]
pub struct BenchResults {
    pub version: String,
    pub timestamp: String,
    pub scenarios: Vec<ScenarioResult>,
    pub totals: Totals,
}

/// Result for a single benchmark scenario.
#[derive(Debug, serde::Serialize)]
pub struct ScenarioResult {
    pub name: String,
    pub command_kind: String,
    pub original_tokens: usize,
    pub filtered_tokens: usize,
    pub savings_tokens: usize,
    #[serde(serialize_with = "serialize_f64")]
    pub savings_percent: f64,
    #[serde(serialize_with = "serialize_f64")]
    pub latency_us: f64,
    pub original_lines: usize,
    pub filtered_lines: usize,
    pub signal_preserved: bool,
}

/// Aggregate totals.
#[derive(Debug, serde::Serialize)]
pub struct Totals {
    pub total_original_tokens: usize,
    pub total_filtered_tokens: usize,
    pub total_savings: usize,
    #[serde(serialize_with = "serialize_f64")]
    pub overall_savings_percent: f64,
    #[serde(serialize_with = "serialize_f64")]
    pub avg_latency_us: f64,
    pub zero_false_positives: bool,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_f64<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64((value * 10.0).round() / 10.0)
}

/// Run all benchmark scenarios.
#[must_use]
pub fn run_all() -> BenchResults {
    let scenarios_def = corpus::all_scenarios();
    let mut results = Vec::with_capacity(scenarios_def.len());

    for scenario in &scenarios_def {
        let pipeline = build_bench_pipeline(&scenario.kind);

        let start = Instant::now();
        let pipe_result = pipeline.process(&scenario.input);
        let elapsed = start.elapsed();

        let original_lines = scenario.input.lines().count();
        let filtered_lines = pipe_result.output.lines().count();

        // Verify zero false positives: all required signal strings must be present
        let signal_preserved = scenario
            .required_signals
            .iter()
            .all(|sig| pipe_result.output.contains(sig));

        results.push(ScenarioResult {
            name: scenario.name.clone(),
            command_kind: format!("{:?}", scenario.kind),
            original_tokens: pipe_result.original_tokens,
            filtered_tokens: pipe_result.filtered_tokens,
            savings_tokens: pipe_result.savings,
            savings_percent: pipe_result.savings_percent(),
            #[allow(clippy::cast_precision_loss)]
            latency_us: elapsed.as_micros() as f64,
            original_lines,
            filtered_lines,
            signal_preserved,
        });
    }

    let total_original: usize = results.iter().map(|r| r.original_tokens).sum();
    let total_filtered: usize = results.iter().map(|r| r.filtered_tokens).sum();
    let total_savings = total_original.saturating_sub(total_filtered);
    let avg_latency = if results.is_empty() {
        0.0
    } else {
        #[allow(clippy::cast_precision_loss)]
        {
            results.iter().map(|r| r.latency_us).sum::<f64>() / results.len() as f64
        }
    };

    #[allow(clippy::cast_precision_loss)]
    let overall_pct = if total_original == 0 {
        0.0
    } else {
        (total_savings as f64 / total_original as f64) * 100.0
    };

    BenchResults {
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        scenarios: results,
        totals: Totals {
            total_original_tokens: total_original,
            total_filtered_tokens: total_filtered,
            total_savings,
            overall_savings_percent: overall_pct,
            avg_latency_us: avg_latency,
            zero_false_positives: scenarios_def
                .iter()
                .zip(
                    // We need to re-check since results moved
                    scenarios_def.iter().map(|s| {
                        let p = build_bench_pipeline(&s.kind);
                        let r = p.process(&s.input);
                        s.required_signals.iter().all(|sig| r.output.contains(sig))
                    }),
                )
                .all(|(_, preserved)| preserved),
        },
    }
}

/// Print human-readable summary.
pub fn print_summary(results: &BenchResults) {
    println!("cli-denoiser benchmark v{}\n", results.version);
    println!(
        "  {:<30} {:>8} {:>8} {:>7} {:>8} {:>6}",
        "Scenario", "Original", "Filtered", "Saved", "Savings", "Signal"
    );
    println!("  {}", "-".repeat(75));

    for s in &results.scenarios {
        let signal = if s.signal_preserved { "OK" } else { "FAIL" };
        println!(
            "  {:<30} {:>8} {:>8} {:>7} {:>7.1}% {:>6}",
            s.name,
            s.original_tokens,
            s.filtered_tokens,
            s.savings_tokens,
            s.savings_percent,
            signal
        );
    }

    println!("  {}", "-".repeat(75));
    println!(
        "  {:<30} {:>8} {:>8} {:>7} {:>7.1}% {:>6}",
        "TOTAL",
        results.totals.total_original_tokens,
        results.totals.total_filtered_tokens,
        results.totals.total_savings,
        results.totals.overall_savings_percent,
        if results.totals.zero_false_positives {
            "OK"
        } else {
            "FAIL"
        }
    );
    println!(
        "\n  Avg latency: {:.0}us | Zero false positives: {}",
        results.totals.avg_latency_us, results.totals.zero_false_positives
    );
}

fn build_bench_pipeline(kind: &CommandKind) -> Pipeline {
    let mut pipeline = Pipeline::new();
    pipeline.add_filter(Box::new(AnsiFilter));
    pipeline.add_filter(Box::new(ProgressFilter));
    pipeline.add_filter(Box::new(DedupFilter::new()));

    let specific: Box<dyn Filter> = match kind {
        CommandKind::Git => Box::new(GitFilter),
        CommandKind::Npm => Box::new(NpmFilter),
        CommandKind::Cargo => Box::new(CargoFilter),
        CommandKind::Docker => Box::new(DockerFilter),
        CommandKind::Kubectl => Box::new(KubectlFilter),
        CommandKind::Unknown => Box::new(GenericFilter),
    };
    pipeline.add_filter(specific);
    pipeline
}
