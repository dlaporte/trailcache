//! Utility functions for string formatting and manipulation.

pub mod format;

// Re-export commonly used functions at module level
pub use format::{cmp_ignore_case, contains_ignore_case, strip_html, truncate};
