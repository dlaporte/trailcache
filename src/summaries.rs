//! AI-generated requirement summaries for merit badges.
//!
//! This module loads pre-generated 40-character summaries for merit badge
//! requirements, providing concise descriptions that fit in the TUI display.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;
use tracing::debug;

/// Global summaries cache, loaded once at startup
static SUMMARIES: OnceLock<SummaryData> = OnceLock::new();

#[derive(Debug, Deserialize, Default)]
struct SummaryFile {
    #[serde(default)]
    version: String,
    #[serde(default)]
    generated: String,
    #[serde(default)]
    summaries: HashMap<String, String>,
    #[serde(default)]
    flags: HashMap<String, String>,
}

#[derive(Debug, Default)]
struct SummaryData {
    summaries: HashMap<String, String>,
}

/// Initialize the summaries from the embedded JSON file.
/// Call this once at app startup.
pub fn init() {
    let _ = SUMMARIES.get_or_init(|| {
        load_summaries().unwrap_or_default()
    });
}

fn load_summaries() -> Option<SummaryData> {
    // Try to load from embedded data first (compile-time include)
    #[cfg(feature = "embedded-summaries")]
    {
        let data = include_str!("../data/requirement_summaries.json");
        if let Ok(file) = serde_json::from_str::<SummaryFile>(data) {
            debug!(
                version = %file.version,
                generated = %file.generated,
                count = file.summaries.len(),
                "Loaded embedded summaries"
            );
            return Some(SummaryData {
                summaries: file.summaries,
            });
        }
    }

    // Fall back to loading from disk at runtime
    let paths = [
        "data/requirement_summaries.json",
        "./data/requirement_summaries.json",
        "../data/requirement_summaries.json",
    ];

    for path in paths {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(file) = serde_json::from_str::<SummaryFile>(&data) {
                debug!(
                    path = %path,
                    version = %file.version,
                    generated = %file.generated,
                    count = file.summaries.len(),
                    "Loaded summaries from disk"
                );
                return Some(SummaryData {
                    summaries: file.summaries,
                });
            }
        }
    }

    debug!("No summaries file found, using original requirement text");
    None
}

/// Get a summary for a requirement text.
/// Returns the AI-generated summary if available, otherwise returns None.
pub fn get_summary(original_text: &str) -> Option<&'static str> {
    SUMMARIES
        .get()
        .and_then(|data| data.summaries.get(original_text))
        .map(|s| s.as_str())
}

/// Get a summary for a requirement text, falling back to the original if no summary exists.
/// Truncates the fallback to max_len if needed.
pub fn get_summary_or_truncate(original_text: &str, max_len: usize) -> String {
    if let Some(summary) = get_summary(original_text) {
        return summary.to_string();
    }

    // Fallback: truncate original text
    if original_text.len() <= max_len {
        original_text.to_string()
    } else {
        let truncated: String = original_text.chars().take(max_len - 1).collect();
        format!("{}…", truncated)
    }
}

/// Check if summaries are loaded
pub fn is_loaded() -> bool {
    SUMMARIES.get().map(|d| !d.summaries.is_empty()).unwrap_or(false)
}

/// Get the number of loaded summaries
pub fn count() -> usize {
    SUMMARIES.get().map(|d| d.summaries.len()).unwrap_or(0)
}
