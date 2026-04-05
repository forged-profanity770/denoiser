#![doc = "CLI Denoiser -- strips terminal noise for LLM agents with zero false positives."]

pub mod bench;
pub mod filters;
pub mod hooks;
pub mod pipeline;
pub mod stream;
pub mod tracker;

pub use pipeline::Pipeline;
