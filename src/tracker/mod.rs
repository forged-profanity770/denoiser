mod db;

pub use db::TrackerDb;

use chrono::Utc;

/// A single recorded filter event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterEvent {
    pub command: String,
    pub original_tokens: usize,
    pub filtered_tokens: usize,
    pub savings: usize,
    pub timestamp: String,
}

impl FilterEvent {
    #[must_use]
    pub fn new(command: &str, original_tokens: usize, filtered_tokens: usize) -> Self {
        Self {
            command: command.to_string(),
            original_tokens,
            filtered_tokens,
            savings: original_tokens.saturating_sub(filtered_tokens),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// Summary stats for the `gain` command.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GainSummary {
    pub total_events: usize,
    pub total_original_tokens: usize,
    pub total_filtered_tokens: usize,
    pub total_savings: usize,
    pub savings_percent: f64,
    pub top_commands: Vec<CommandSavings>,
    pub period_days: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandSavings {
    pub command: String,
    pub events: usize,
    pub savings: usize,
}
