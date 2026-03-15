//! Requirement sorting utilities shared across all interfaces.
//!
//! Sorts requirements numerically by their number field, keeping
//! sub-requirements (e.g., "a", "b") grouped under their parent.

use super::{RankRequirement, MeritBadgeRequirement};

/// Parse a requirement number into a sortable key.
/// "3a" -> (3, "a"), "a" -> (0, "a"), "10" -> (10, "")
pub fn req_number_sort_key(num: &str) -> (u32, String) {
    let trimmed = num.trim_matches(|c: char| c == '(' || c == ')' || c.is_whitespace());
    let numeric_end = trimmed.find(|c: char| !c.is_ascii_digit()).unwrap_or(trimmed.len());
    if numeric_end > 0 {
        let n: u32 = trimmed[..numeric_end].parse().unwrap_or(u32::MAX);
        let suffix = trimmed[numeric_end..].to_lowercase();
        (n, suffix)
    } else {
        // Purely alphabetic — sub-requirement, parent assigned during sort
        (0, trimmed.to_lowercase())
    }
}

/// Given requirement number strings, return indices sorted numerically
/// with sub-requirements grouped under their preceding parent.
pub fn sorted_indices_by_number(numbers: &[String]) -> Vec<usize> {
    let keys: Vec<(u32, String)> = numbers.iter().map(|n| req_number_sort_key(n)).collect();

    // Assign parent numbers to purely-alpha sub-requirements
    let mut parent_map: Vec<u32> = Vec::with_capacity(keys.len());
    let mut last_parent: u32 = 0;
    for key in &keys {
        if key.0 > 0 {
            last_parent = key.0;
            parent_map.push(key.0);
        } else {
            parent_map.push(last_parent);
        }
    }

    let final_keys: Vec<(u32, String)> = keys
        .iter()
        .enumerate()
        .map(|(i, key)| {
            if key.0 == 0 {
                (parent_map[i], key.1.clone())
            } else {
                key.clone()
            }
        })
        .collect();

    let mut indices: Vec<usize> = (0..numbers.len()).collect();
    indices.sort_by(|&a, &b| final_keys[a].cmp(&final_keys[b]));
    indices
}

/// Trait for types that have a requirement number string.
pub trait HasRequirementNumber {
    fn requirement_number_str(&self) -> String;
}

impl HasRequirementNumber for RankRequirement {
    fn requirement_number_str(&self) -> String {
        self.number()
    }
}

impl HasRequirementNumber for MeritBadgeRequirement {
    fn requirement_number_str(&self) -> String {
        self.number()
    }
}

/// Sort a Vec of any type with a requirement number, keeping
/// sub-requirements grouped under their parent.
pub fn sort_requirements<T: HasRequirementNumber + Clone>(reqs: &mut [T]) {
    let numbers: Vec<String> = reqs.iter().map(|r| r.requirement_number_str()).collect();
    let order = sorted_indices_by_number(&numbers);
    let orig = reqs.to_vec();
    for (i, &idx) in order.iter().enumerate() {
        reqs[i] = orig[idx].clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_req_number_sort_key() {
        assert_eq!(req_number_sort_key("1"), (1, String::new()));
        assert_eq!(req_number_sort_key("3a"), (3, "a".to_string()));
        assert_eq!(req_number_sort_key("10"), (10, String::new()));
        assert_eq!(req_number_sort_key("a"), (0, "a".to_string()));
        assert_eq!(req_number_sort_key("(b)"), (0, "b".to_string()));
    }

    #[test]
    fn test_sorted_indices_numeric() {
        let nums: Vec<String> = vec!["3", "1", "2", "10"]
            .into_iter()
            .map(String::from)
            .collect();
        let order = sorted_indices_by_number(&nums);
        let sorted: Vec<&str> = order.iter().map(|&i| nums[i].as_str()).collect();
        assert_eq!(sorted, vec!["1", "2", "3", "10"]);
    }

    #[test]
    fn test_sorted_indices_with_subreqs() {
        let nums: Vec<String> = vec!["1", "a", "b", "2", "a", "3"]
            .into_iter()
            .map(String::from)
            .collect();
        let order = sorted_indices_by_number(&nums);
        let sorted: Vec<&str> = order.iter().map(|&i| nums[i].as_str()).collect();
        assert_eq!(sorted, vec!["1", "a", "b", "2", "a", "3"]);
    }

    #[test]
    fn test_sorted_indices_mixed() {
        let nums: Vec<String> = vec!["2", "1", "1a", "1b", "3", "2a"]
            .into_iter()
            .map(String::from)
            .collect();
        let order = sorted_indices_by_number(&nums);
        let sorted: Vec<&str> = order.iter().map(|&i| nums[i].as_str()).collect();
        assert_eq!(sorted, vec!["1", "1a", "1b", "2", "2a", "3"]);
    }
}
