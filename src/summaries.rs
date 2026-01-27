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
    // Try to load from disk at runtime
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
