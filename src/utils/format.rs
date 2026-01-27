use std::cmp::Ordering;

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
#[allow(dead_code)] // Used in ui::tabs::patrols
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
