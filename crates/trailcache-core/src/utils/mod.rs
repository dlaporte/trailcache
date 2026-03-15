//! Utility functions for string formatting and manipulation.

pub mod format;

// Re-export commonly used functions at module level
pub use format::{
    check_expiration, cmp_ignore_case, contains_ignore_case, format_phone, strip_html,
    strip_url_scheme, truncate, wrap_text, ExpirationStatus,
};
