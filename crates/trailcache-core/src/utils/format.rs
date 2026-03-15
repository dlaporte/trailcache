use std::cmp::Ordering;

use chrono::{NaiveDate, Utc};

// ============================================================================
// Expiration Status
// ============================================================================

/// Classification of a date-based expiration relative to today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpirationStatus {
    Active,
    ExpiringSoon,
    Expired,
}

impl ExpirationStatus {
    /// Format as display text, e.g. "Expired Mar 15, 2026" or "Expires Mar 15, 2026".
    pub fn format_expiry(&self, formatted_date: &str) -> String {
        match self {
            ExpirationStatus::Expired => format!("Expired {}", formatted_date),
            _ => format!("Expires {}", formatted_date),
        }
    }

    /// Format for YPT status (uses "Current" instead of "Expires" for active).
    pub fn format_ypt(&self, formatted_date: &str) -> String {
        match self {
            ExpirationStatus::Expired => format!("Expired {}", formatted_date),
            ExpirationStatus::ExpiringSoon => format!("Expires {}", formatted_date),
            ExpirationStatus::Active => format!("Current ({})", formatted_date),
        }
    }

    /// CSS-style class name for this status.
    pub fn style_class(&self) -> &'static str {
        match self {
            ExpirationStatus::Expired => "expired",
            ExpirationStatus::ExpiringSoon => "expiring",
            ExpirationStatus::Active => "active",
        }
    }

    /// For membership: "current" instead of "active" style class.
    pub fn membership_style_class(&self) -> &'static str {
        match self {
            ExpirationStatus::Active => "current",
            other => other.style_class(),
        }
    }
}

/// Number of days before expiration to flag as "expiring soon".
pub const EXPIRING_SOON_DAYS: i64 = 90;

/// Parse a YYYY-MM-DD date string and classify it as expired / expiring soon / active.
///
/// Returns `(status, formatted_date)` where formatted_date is e.g. "Mar 15, 2026".
/// The "expiring soon" threshold is [`EXPIRING_SOON_DAYS`] days (inclusive).
/// Returns `None` if the date string cannot be parsed.
pub fn check_expiration(date_str: &str) -> Option<(ExpirationStatus, String)> {
    // Take first 10 chars to handle timestamps or extra suffixes
    let date_part = &date_str[..10.min(date_str.len())];
    let date = NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()?;
    let today = Utc::now().date_naive();
    let formatted = date.format("%b %d, %Y").to_string();

    let status = if date < today {
        ExpirationStatus::Expired
    } else if date <= today + chrono::Duration::days(EXPIRING_SOON_DAYS) {
        ExpirationStatus::ExpiringSoon
    } else {
        ExpirationStatus::Active
    };

    Some((status, formatted))
}

// ============================================================================
// String Comparison Utilities
// ============================================================================

/// Case-insensitive substring check without allocation.
/// Assumes `needle` is already lowercase.
pub fn contains_ignore_case(haystack: &str, needle_lowercase: &str) -> bool {
    if needle_lowercase.is_empty() {
        return true;
    }
    haystack
        .char_indices()
        .any(|(i, _)| {
            haystack[i..]
                .chars()
                .zip(needle_lowercase.chars())
                .all(|(h, n)| h.to_ascii_lowercase() == n)
                && haystack[i..].chars().count() >= needle_lowercase.chars().count()
        })
}

/// Case-insensitive string comparison for sorting (no allocation).
pub fn cmp_ignore_case(a: &str, b: &str) -> Ordering {
    a.chars()
        .map(|c| c.to_ascii_lowercase())
        .cmp(b.chars().map(|c| c.to_ascii_lowercase()))
}

// ============================================================================
// Phone Number Formatting
// ============================================================================

/// Format a phone number for display
/// Handles various input formats and normalizes to (XXX) XXX-XXXX
pub fn format_phone(phone: &str) -> String {
    // Extract just the digits
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    match digits.len() {
        10 => format!(
            "({}) {}-{}",
            &digits[0..3],
            &digits[3..6],
            &digits[6..10]
        ),
        11 if digits.starts_with('1') => format!(
            "({}) {}-{}",
            &digits[1..4],
            &digits[4..7],
            &digits[7..11]
        ),
        _ => phone.to_string(), // Return original if can't format
    }
}

/// Strip HTML tags from a string.
/// Useful for cleaning up requirement text from the API.
pub fn strip_html(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    // Normalize whitespace
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Truncate a string to a maximum length, adding ellipsis if needed.
/// Handles tabs by replacing with spaces and trims whitespace.
pub fn truncate(s: &str, max_len: usize) -> String {
    // Replace tabs with spaces and trim to avoid display width issues
    let cleaned: String = s.replace('\t', " ").trim().to_string();
    if cleaned.len() <= max_len {
        cleaned
    } else if max_len <= 1 {
        cleaned.chars().take(max_len).collect()
    } else {
        format!("{}…", &cleaned[..max_len.saturating_sub(1)])
    }
}

/// Word-wrap text into lines of at most `width` characters, breaking at word boundaries.
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut result = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            if word.len() > width {
                let mut remaining = word;
                while remaining.len() > width {
                    result.push(remaining[..width].to_string());
                    remaining = &remaining[width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        } else if current_line.len() + 1 + word.len() > width {
            result.push(current_line);
            if word.len() > width {
                let mut remaining = word;
                while remaining.len() > width {
                    result.push(remaining[..width].to_string());
                    remaining = &remaining[width..];
                }
                current_line = remaining.to_string();
            } else {
                current_line = word.to_string();
            }
        } else {
            current_line.push(' ');
            current_line.push_str(word);
        }
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

/// Strip the URL scheme (http:// or https://) from a URL for display.
pub fn strip_url_scheme(url: &str) -> &str {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
}

/// Format an optional string, returning a default if None
#[allow(dead_code)]
pub fn format_optional(value: &Option<String>, default: &str) -> String {
    value.as_deref().unwrap_or(default).to_string()
}

/// Format a date string to a more readable format
#[allow(dead_code)]
pub fn format_date(date: &str) -> String {
    // Try to parse ISO format and convert to readable
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date) {
        dt.format("%b %d, %Y").to_string()
    } else if date.len() >= 10 {
        // Try to parse YYYY-MM-DD format
        date.chars().take(10).collect()
    } else {
        date.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_phone() {
        assert_eq!(format_phone("5551234567"), "(555) 123-4567");
        assert_eq!(format_phone("15551234567"), "(555) 123-4567");
        assert_eq!(format_phone("555-123-4567"), "(555) 123-4567");
        assert_eq!(format_phone("(555) 123-4567"), "(555) 123-4567");
        assert_eq!(format_phone("123"), "123"); // Too short, return as-is
    }

    #[test]
    fn test_check_expiration_expired() {
        let past = "2020-01-01";
        let (status, formatted) = check_expiration(past).unwrap();
        assert_eq!(status, ExpirationStatus::Expired);
        assert_eq!(formatted, "Jan 01, 2020");
    }

    #[test]
    fn test_check_expiration_active() {
        // Far future date
        let future = "2099-12-31";
        let (status, _) = check_expiration(future).unwrap();
        assert_eq!(status, ExpirationStatus::Active);
    }

    #[test]
    fn test_check_expiration_invalid() {
        assert!(check_expiration("not-a-date").is_none());
        assert!(check_expiration("").is_none());
    }

    #[test]
    fn test_check_expiration_with_timestamp_suffix() {
        let (status, _) = check_expiration("2020-01-01T00:00:00Z").unwrap();
        assert_eq!(status, ExpirationStatus::Expired);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("Hello", 10), "Hello");
        assert_eq!(truncate("Hello World", 8), "Hello W…");
        assert_eq!(truncate("Hi", 2), "Hi");
        // Test tab handling
        assert_eq!(truncate("Hello\tWorld", 20), "Hello World");
        // Test trimming
        assert_eq!(truncate("  Hello  ", 10), "Hello");
    }
}
