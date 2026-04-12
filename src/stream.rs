use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;

use crate::pipeline::{Pipeline, PipelineResult};

/// Execute a command, capture output, filter through pipeline, return result.
/// Preserves the original exit code.
///
/// # Errors
/// Returns `StreamError` if the command cannot be spawned or waited on.
pub async fn run_filtered(
    command: &str,
    args: &[String],
    pipeline: &Pipeline,
) -> Result<FilteredRun, StreamError> {
    let mut child = Command::new(command)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| StreamError::SpawnFailed {
            command: command.to_string(),
            source: e,
        })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let (stdout_lines, stderr_lines) =
        tokio::join!(read_lines_from(stdout), read_lines_from(stderr),);

    let status = child.wait().await.map_err(|e| StreamError::WaitFailed {
        command: command.to_string(),
        source: e,
    })?;

    let raw_stdout = stdout_lines.join("\n");
    let raw_stderr = stderr_lines.join("\n");

    let stdout_result = pipeline.process(&raw_stdout);
    let stderr_result = pipeline.process(&raw_stderr);

    Ok(FilteredRun {
        stdout: stdout_result,
        stderr: stderr_result,
        exit_code: status.code().unwrap_or(-1),
        raw_stdout_len: raw_stdout.len(),
        raw_stderr_len: raw_stderr.len(),
    })
}

async fn read_lines_from<R: AsyncRead + Unpin>(reader: Option<R>) -> Vec<String> {
    let Some(reader) = reader else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    while buf_reader.read_line(&mut line).await.unwrap_or(0) > 0 {
        lines.push(line.trim_end_matches('\n').to_string());
        line.clear();
    }
    lines
}

#[derive(Debug)]
pub struct FilteredRun {
    pub stdout: PipelineResult,
    pub stderr: PipelineResult,
    pub exit_code: i32,
    pub raw_stdout_len: usize,
    pub raw_stderr_len: usize,
}

impl FilteredRun {
    #[must_use]
    pub fn total_savings(&self) -> usize {
        self.stdout.savings + self.stderr.savings
    }

    #[must_use]
    pub fn total_original_tokens(&self) -> usize {
        self.stdout.original_tokens + self.stderr.original_tokens
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("failed to spawn '{command}': {source}")]
    SpawnFailed {
        command: String,
        source: std::io::Error,
    },
    #[error("failed waiting for '{command}': {source}")]
    WaitFailed {
        command: String,
        source: std::io::Error,
    },
}
