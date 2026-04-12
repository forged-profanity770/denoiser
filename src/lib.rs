#![doc = "CLI Denoiser -- strips terminal noise for LLM agents with zero false positives."]

pub mod bench;
pub mod filters;
pub mod hooks;
pub mod pipeline;
pub mod stream;
pub mod tracker;

pub use pipeline::Pipeline;

use filters::ansi::AnsiFilter;
use filters::cargo::CargoFilter;
use filters::dedup::DedupFilter;
use filters::docker::DockerFilter;
use filters::generic::GenericFilter;
use filters::git::GitFilter;
use filters::kubectl::KubectlFilter;
use filters::npm::NpmFilter;
use filters::progress::ProgressFilter;
use filters::{CommandKind, Filter};

#[must_use]
pub fn build_pipeline(kind: &CommandKind, debug: bool) -> Pipeline {
    let mut pipeline = Pipeline::new();
    pipeline.set_debug(debug);

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
