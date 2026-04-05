use std::path::PathBuf;

use rusqlite::Connection;

use super::{CommandSavings, DailyStats, FilterEvent, GainSummary};

const DB_FILENAME: &str = "cli-denoiser.db";
const RETENTION_DAYS: u32 = 90;

/// `SQLite`-backed token savings tracker.
pub struct TrackerDb {
    conn: Connection,
}

impl TrackerDb {
    /// Open or create the tracker database in the user's data directory.
    ///
    /// # Errors
    /// Returns `TrackerError` if the data directory is unavailable or `SQLite` fails.
    pub fn open() -> Result<Self, TrackerError> {
        let path = db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| TrackerError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let conn = Connection::open(&path).map_err(|e| TrackerError::Sqlite {
            context: "open database".to_string(),
            source: e,
        })?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command TEXT NOT NULL,
                original_tokens INTEGER NOT NULL,
                filtered_tokens INTEGER NOT NULL,
                savings INTEGER NOT NULL,
                timestamp TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_command ON events(command);",
        )
        .map_err(|e| TrackerError::Sqlite {
            context: "create tables".to_string(),
            source: e,
        })?;

        Ok(Self { conn })
    }

    /// Record a filter event.
    ///
    /// # Errors
    /// Returns `TrackerError` if the insert fails.
    pub fn record(&self, event: &FilterEvent) -> Result<(), TrackerError> {
        self.conn
            .execute(
                "INSERT INTO events (command, original_tokens, filtered_tokens, savings, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    event.command,
                    event.original_tokens,
                    event.filtered_tokens,
                    event.savings,
                    event.timestamp,
                ],
            )
            .map_err(|e| TrackerError::Sqlite {
                context: "insert event".to_string(),
                source: e,
            })?;
        Ok(())
    }

    /// Get savings summary for the last N days.
    ///
    /// # Errors
    /// Returns `TrackerError` if the query fails.
    #[allow(clippy::cast_precision_loss)]
    pub fn gain_summary(&self, days: u32) -> Result<GainSummary, TrackerError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(days));
        let cutoff_str = cutoff.to_rfc3339();

        let mut stmt = self
            .conn
            .prepare(
                "SELECT COUNT(*), COALESCE(SUM(original_tokens), 0),
                        COALESCE(SUM(filtered_tokens), 0), COALESCE(SUM(savings), 0)
                 FROM events WHERE timestamp >= ?1",
            )
            .map_err(|e| TrackerError::Sqlite {
                context: "prepare summary".to_string(),
                source: e,
            })?;

        let (total_events, total_original, total_filtered, total_savings): (
            usize,
            usize,
            usize,
            usize,
        ) = stmt
            .query_row(rusqlite::params![cutoff_str], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| TrackerError::Sqlite {
                context: "query summary".to_string(),
                source: e,
            })?;

        let savings_percent = if total_original == 0 {
            0.0
        } else {
            (total_savings as f64 / total_original as f64) * 100.0
        };

        // Top commands by savings
        let mut cmd_stmt = self
            .conn
            .prepare(
                "SELECT command, COUNT(*), SUM(savings)
                 FROM events WHERE timestamp >= ?1
                 GROUP BY command ORDER BY SUM(savings) DESC LIMIT 10",
            )
            .map_err(|e| TrackerError::Sqlite {
                context: "prepare top commands".to_string(),
                source: e,
            })?;

        let top_commands: Vec<CommandSavings> = cmd_stmt
            .query_map(rusqlite::params![cutoff_str], |row| {
                Ok(CommandSavings {
                    command: row.get(0)?,
                    events: row.get(1)?,
                    savings: row.get(2)?,
                })
            })
            .map_err(|e| TrackerError::Sqlite {
                context: "query top commands".to_string(),
                source: e,
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(GainSummary {
            total_events,
            total_original_tokens: total_original,
            total_filtered_tokens: total_filtered,
            total_savings,
            savings_percent,
            top_commands,
            period_days: days,
        })
    }

    /// Get daily breakdown for the last N days.
    ///
    /// # Errors
    /// Returns `TrackerError` if the query fails.
    #[allow(clippy::cast_precision_loss)]
    pub fn daily_report(&self, days: u32) -> Result<Vec<DailyStats>, TrackerError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(days));
        let cutoff_str = cutoff.to_rfc3339();

        let mut stmt = self
            .conn
            .prepare(
                "SELECT DATE(timestamp) as day,
                        COUNT(*),
                        SUM(original_tokens),
                        SUM(filtered_tokens),
                        SUM(savings)
                 FROM events
                 WHERE timestamp >= ?1
                 GROUP BY day
                 ORDER BY day DESC",
            )
            .map_err(|e| TrackerError::Sqlite {
                context: "prepare daily report".to_string(),
                source: e,
            })?;

        let rows: Vec<DailyStats> = stmt
            .query_map(rusqlite::params![cutoff_str], |row| {
                let original: usize = row.get(2)?;
                let filtered: usize = row.get(3)?;
                let savings: usize = row.get(4)?;
                let pct = if original == 0 {
                    0.0
                } else {
                    (savings as f64 / original as f64) * 100.0
                };
                Ok(DailyStats {
                    date: row.get(0)?,
                    events: row.get(1)?,
                    original_tokens: original,
                    filtered_tokens: filtered,
                    savings,
                    savings_percent: pct,
                })
            })
            .map_err(|e| TrackerError::Sqlite {
                context: "query daily report".to_string(),
                source: e,
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
    }

    /// Get recent event log (last N events).
    ///
    /// # Errors
    /// Returns `TrackerError` if the query fails.
    pub fn recent_events(&self, limit: u32) -> Result<Vec<FilterEvent>, TrackerError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT command, original_tokens, filtered_tokens, savings, timestamp
                 FROM events ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| TrackerError::Sqlite {
                context: "prepare recent events".to_string(),
                source: e,
            })?;

        let rows: Vec<FilterEvent> = stmt
            .query_map(rusqlite::params![limit], |row| {
                Ok(FilterEvent {
                    command: row.get(0)?,
                    original_tokens: row.get(1)?,
                    filtered_tokens: row.get(2)?,
                    savings: row.get(3)?,
                    timestamp: row.get(4)?,
                })
            })
            .map_err(|e| TrackerError::Sqlite {
                context: "query recent events".to_string(),
                source: e,
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
    }

    /// Prune events older than retention period.
    ///
    /// # Errors
    /// Returns `TrackerError` if the delete fails.
    pub fn prune(&self) -> Result<usize, TrackerError> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(RETENTION_DAYS));
        let cutoff_str = cutoff.to_rfc3339();

        let deleted = self
            .conn
            .execute(
                "DELETE FROM events WHERE timestamp < ?1",
                rusqlite::params![cutoff_str],
            )
            .map_err(|e| TrackerError::Sqlite {
                context: "prune old events".to_string(),
                source: e,
            })?;

        Ok(deleted)
    }
}

fn db_path() -> Result<PathBuf, TrackerError> {
    let data_dir = dirs::data_local_dir().ok_or(TrackerError::NoDataDir)?;
    Ok(data_dir.join("cli-denoiser").join(DB_FILENAME))
}

#[derive(Debug, thiserror::Error)]
pub enum TrackerError {
    #[error("no local data directory found")]
    NoDataDir,
    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("SQLite error ({context}): {source}")]
    Sqlite {
        context: String,
        source: rusqlite::Error,
    },
}
