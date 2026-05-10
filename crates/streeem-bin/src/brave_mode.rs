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

/// Look at the visible cells of a tile. If a Claude permission prompt with
/// numbered options is on screen, decide what to send.
///
/// A "prompt" is detected when the buffer has, in close proximity:
/// - At least one line containing the `❯` arrow character (Claude's selection
///   cursor), OR a line ending with `?` within 5 lines above the first
///   numbered option, AND
/// - Two or more lines that match the numbered-option pattern
///   (e.g., `1. ...`, `2. ...`, optionally prefixed by `❯` and whitespace).
///
/// The numbered options are scanned across the WHOLE buffer (not just lines
/// after a specific phrase), since Claude may render the question and options
/// in different orders or with intermediate blank lines.
///
/// Returns `Some(response)` only if the detected prompt's hash differs from
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

    // Find all numbered-option lines (e.g., "1. Yes", "❯ 1. Yes", "  2. No").
    let mut numbered: Vec<(usize, u8, String)> = Vec::new(); // (line_idx, number, text-after-"N. ")
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start_matches('❯').trim_start();
        let bytes = trimmed.as_bytes();
        if bytes.len() >= 3 && bytes[0].is_ascii_digit() && bytes[1] == b'.' && bytes[2] == b' ' {
            let n = bytes[0] - b'0';
            numbered.push((i, n, trimmed[3..].to_string()));
        }
    }

    // Need at least options 1 and 2 to call this a prompt.
    if numbered.len() < 2 {
        return None;
    }
    let first_two_present =
        numbered.iter().any(|(_, n, _)| *n == 1) && numbered.iter().any(|(_, n, _)| *n == 2);
    if !first_two_present {
        return None;
    }

    // Confirm there's a `❯` arrow on screen anywhere (Claude's cursor marker)
    // OR a line containing "?" within 5 lines above the first numbered option.
    let first_opt_idx = numbered
        .iter()
        .find(|(_, n, _)| *n == 1)
        .map(|(i, _, _)| *i)?;
    let has_arrow = lines.iter().any(|l| l.contains('❯'));
    let has_question_nearby = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < first_opt_idx && first_opt_idx - *i <= 5)
        .any(|(_, l)| l.contains('?'));

    if !has_arrow && !has_question_nearby {
        return None;
    }

    // Build options list in numeric order (only 1, 2, 3 — first occurrence of each).
    let mut options: Vec<String> = Vec::new();
    for n in 1..=9u8 {
        if let Some((_, _, text)) = numbered.iter().find(|(_, num, _)| *num == n) {
            options.push(text.clone());
        } else {
            break;
        }
    }

    if options.len() < 2 {
        return None;
    }

    // Hash the prompt context (the lines around the first option) to dedupe.
    let mut hasher = DefaultHasher::new();
    let context_start = first_opt_idx.saturating_sub(3);
    let context_end = (first_opt_idx + options.len()).min(lines.len());
    for line in &lines[context_start..context_end] {
        line.hash(&mut hasher);
    }
    let prompt_hash = hasher.finish();
    if Some(prompt_hash) == last_responded_hash {
        return None;
    }

    // Choose the response.
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

    #[test]
    fn detects_prompt_without_proceed_phrase_when_arrow_present() {
        // Claude's actual prompt may not literally say "Do you want to proceed?".
        let c = cells_from(
            &[
                "Bash command",
                "  echo hello",
                "  Print a greeting",
                "",
                "❯ 1. Yes",
                "  2. No",
            ],
            60,
        );
        let r = detect(&c, None).unwrap();
        assert_eq!(r.bytes, b"\r".to_vec());
    }

    #[test]
    fn requires_at_least_two_options() {
        let c = cells_from(&["❯ 1. Only one option here"], 60);
        assert!(detect(&c, None).is_none());
    }

    #[test]
    fn detects_dont_ask_again_in_alternate_phrasing() {
        let c = cells_from(
            &[
                "Run this command?",
                "❯ 1. Yes",
                "  2. Yes, and don't ask again for: git status",
                "  3. No",
            ],
            80,
        );
        let r = detect(&c, None).unwrap();
        assert_eq!(r.bytes, b"2\r".to_vec());
    }

    #[test]
    fn ignores_numbered_lines_without_arrow_or_question() {
        // A code listing has "1. foo / 2. bar" but no ❯ and no ? — not a prompt.
        let c = cells_from(
            &[
                "Steps to follow",
                "  1. install deps",
                "  2. run tests",
                "  3. ship it",
            ],
            60,
        );
        assert!(detect(&c, None).is_none());
    }
}
