pub mod signing;
pub mod typed_data;

use alloy_primitives::{Address, Bytes, Signature, B256};
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DigestRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub signer: Address,
    pub signature: Signature,
    pub typed_data_concat: Bytes,
    pub digest_range: DigestRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub signer: Address,
    pub digest: B256,
}

/// Find byte ranges of concatenated JSON objects within a single string by matching braces.
/// - Handles nested objects
/// - Ignores braces that appear inside JSON strings (with escape handling)
/// Returns ranges as [start, end) byte offsets into the original string.
pub fn find_concatenated_json_ranges(input: &str) -> Result<Vec<DigestRange>> {
    let mut ranges: Vec<DigestRange> = Vec::new();
    let mut depth: u32 = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut current_start: Option<usize> = None;

    for (idx, ch) in input.char_indices() {
        if in_string {
            if escape {
                // Current character is escaped; do not interpret it
                escape = false;
                continue;
            }
            match ch {
                '\\' => {
                    escape = true;
                }
                '"' => {
                    in_string = false;
                }
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
            }
            '{' => {
                if depth == 0 {
                    current_start = Some(idx);
                }
                depth = depth.saturating_add(1);
            }
            '}' => {
                if depth == 0 {
                    return Err(anyhow!("Unmatched closing brace at byte {idx}"));
                }
                depth -= 1;
                if depth == 0 {
                    let start = current_start.ok_or_else(|| anyhow!("Missing start for JSON object ending at byte {idx}"))?;
                    // end is exclusive; include this '}'
                    let end = idx + ch.len_utf8();
                    ranges.push(DigestRange { start, end });
                    current_start = None;
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(anyhow!("Unclosed JSON object(s); brace depth at end is {depth}"));
    }

    Ok(ranges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_simple_object() {
        let s = r#"{"a":1}"#;
        let ranges = find_concatenated_json_ranges(s).unwrap();
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0], DigestRange { start: 0, end: s.len() });
        assert_eq!(&s[ranges[0].start..ranges[0].end], s);
    }

    #[test]
    fn multiple_concatenated_objects() {
        let s = r#"{"a":1}{"b":2}{"c":3}"#;
        let ranges = find_concatenated_json_ranges(s).unwrap();
        assert_eq!(ranges.len(), 3);
        let parts: Vec<&str> = ranges
            .iter()
            .map(|r| &s[r.start..r.end])
            .collect();
        assert_eq!(parts[0], "{\"a\":1}");
        assert_eq!(parts[1], "{\"b\":2}");
        assert_eq!(parts[2], "{\"c\":3}");
    }

    #[test]
    fn nested_objects() {
        let s = r#"{"a":{"b":2},"c":3}{"d":4}"#;
        let ranges = find_concatenated_json_ranges(s).unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(&s[ranges[0].start..ranges[0].end], "{\"a\":{\"b\":2},\"c\":3}");
        assert_eq!(&s[ranges[1].start..ranges[1].end], "{\"d\":4}");
    }

    #[test]
    fn braces_inside_strings_are_ignored() {
        let s = r#"{"a":"{not a brace}","b":1}{"c":"}\"}"}"#;
        let ranges = find_concatenated_json_ranges(s).unwrap();
        assert_eq!(ranges.len(), 2);
        assert_eq!(&s[ranges[0].start..ranges[0].end], "{\"a\":\"{not a brace}\",\"b\":1}");
        assert_eq!(&s[ranges[1].start..ranges[1].end], "{\"c\":\"}\\\"}\"}");
    }

    #[test]
    fn unmatched_closing_brace_errors() {
        let s = "}";
        let err = find_concatenated_json_ranges(s).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Unmatched closing brace"));
    }

    #[test]
    fn unclosed_object_errors() {
        let s = "{";
        let err = find_concatenated_json_ranges(s).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Unclosed JSON object"));
    }
}
