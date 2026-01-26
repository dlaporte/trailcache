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

/// Truncate a string to a maximum length, adding ellipsis if needed
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s.chars().take(max_len).collect()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// Format an optional string, returning a default if None
pub fn format_optional(value: &Option<String>, default: &str) -> String {
    value.as_deref().unwrap_or(default).to_string()
}

/// Format a date string to a more readable format
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
    fn test_truncate_string() {
        assert_eq!(truncate_string("Hello", 10), "Hello");
        assert_eq!(truncate_string("Hello World", 8), "Hello...");
        assert_eq!(truncate_string("Hi", 2), "Hi");
    }
}
