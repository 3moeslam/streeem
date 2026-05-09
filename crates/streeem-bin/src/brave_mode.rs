//! Detects Claude-style permission prompts in a tile's buffer and computes
//! the bytes to write back to auto-confirm them.
//!
//! Returns None when no prompt is detected (or when the same prompt has
//! already been auto-confirmed). The caller passes the previous response's
//! checksum so we don't fire twice on the same on-screen prompt.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use streeem_domain::terminal_buffer::Cell;

pub struct BraveResponse {
    pub bytes: Vec<u8>,
    pub prompt_hash: u64,
}

/// Look at the visible cells of a tile. If a "Do you want to proceed?"
/// prompt with numbered options is on screen, decide what to send.
///
/// Logic:
/// - If we find a line starting with "Do you want to proceed?", scan the
///   following lines for numbered options ("1.", "2.", ...).
/// - 2 options total: send "\r" (Enter, picks the highlighted option 1).
/// - 3+ options where option 2 starts with "Yes, and don't ask again":
///   send "2\r" to pick that option.
/// - Otherwise: send "\r" (default to option 1).
///
/// Returns Some(response) only if the detected prompt's hash differs from
/// `last_responded_hash`.
pub fn detect(cells: &[Vec<Cell>], last_responded_hash: Option<u64>) -> Option<BraveResponse> {
    let lines: Vec<String> = cells
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| c.ch)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();

    // Find "Do you want to proceed?" line.
    let prompt_idx = lines
        .iter()
        .position(|l| l.contains("Do you want to proceed?"))?;

    // Collect numbered options that appear after the prompt.
    let mut options: Vec<String> = Vec::new();
    for line in lines.iter().skip(prompt_idx + 1) {
        let trimmed = line.trim_start_matches('❯').trim();
        // Match "1. ", "2. ", "3. ", etc. (single digit only — Claude prompts are short).
        if trimmed.len() >= 3 {
            let bytes = trimmed.as_bytes();
            if bytes[0].is_ascii_digit() && bytes[1] == b'.' && bytes[2] == b' ' {
                options.push(trimmed[3..].to_string());
                continue;
            }
        }
        // If we already started collecting options and hit a non-option line, stop.
        if !options.is_empty() {
            break;
        }
    }

    if options.is_empty() {
        return None;
    }

    // Hash the prompt context to dedupe.
    let mut hasher = DefaultHasher::new();
    lines[prompt_idx].hash(&mut hasher);
    for opt in &options {
        opt.hash(&mut hasher);
    }
    let prompt_hash = hasher.finish();
    if Some(prompt_hash) == last_responded_hash {
        return None;
    }

    // Decide which option to send.
    let bytes = if options.len() >= 3 && options[1].to_ascii_lowercase().starts_with("yes, and don")
    {
        b"2\r".to_vec()
    } else {
        b"\r".to_vec()
    };

    Some(BraveResponse { bytes, prompt_hash })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use streeem_domain::terminal_buffer::Cell;

    fn cells_from(lines: &[&str], width: usize) -> Vec<Vec<Cell>> {
        lines
            .iter()
            .map(|line| {
                let mut row: Vec<Cell> = line
                    .chars()
                    .map(|c| Cell {
                        ch: c,
                        ..Cell::default()
                    })
                    .collect();
                while row.len() < width {
                    row.push(Cell::default());
                }
                row
            })
            .collect()
    }

    #[test]
    fn no_prompt_returns_none() {
        let c = cells_from(&["just running normal output", ""], 60);
        assert!(detect(&c, None).is_none());
    }

    #[test]
    fn two_option_prompt_sends_enter() {
        let c = cells_from(
            &[
                "Some context line",
                "Do you want to proceed?",
                "❯ 1. Yes",
                "  2. No",
            ],
            60,
        );
        let r = detect(&c, None).unwrap();
        assert_eq!(r.bytes, b"\r".to_vec());
    }

    #[test]
    fn three_option_with_dont_ask_again_sends_2_enter() {
        let c = cells_from(
            &[
                "Do you want to proceed?",
                "❯ 1. Yes",
                "  2. Yes, and don't ask again for: ./gradlew tasks *",
                "  3. No",
            ],
            80,
        );
        let r = detect(&c, None).unwrap();
        assert_eq!(r.bytes, b"2\r".to_vec());
    }

    #[test]
    fn three_option_without_dont_ask_again_falls_back_to_enter() {
        let c = cells_from(
            &[
                "Do you want to proceed?",
                "❯ 1. Yes",
                "  2. Yes once",
                "  3. No",
            ],
            60,
        );
        let r = detect(&c, None).unwrap();
        assert_eq!(r.bytes, b"\r".to_vec());
    }

    #[test]
    fn second_call_with_same_hash_returns_none() {
        let c = cells_from(&["Do you want to proceed?", "❯ 1. Yes", "  2. No"], 60);
        let first = detect(&c, None).unwrap();
        assert!(detect(&c, Some(first.prompt_hash)).is_none());
    }
}
