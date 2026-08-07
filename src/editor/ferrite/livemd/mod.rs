//! Live inline (Typora-style) WYSIWYG markdown mode — pure-logic core.
//!
//! See `.claude/skills/livemd/SKILL.md` for the architecture contract.
//!
//! `block`, `scan` and `map` are pure logic with no egui types: they are
//! unit-testable without a GUI context, and that is where the correctness
//! risk for this feature lives. A later phase adds `style.rs` (egui
//! `TextFormat` mapping) and wires this module into the editor render loop.

pub mod block;
pub mod map;
pub mod scan;
pub mod style;

pub use block::{compute_block_contexts, BlockContext};
pub use map::LineMap;
pub use scan::scan_line;

/// Converts a char index within `s` to a byte offset. Total: an out-of-range
/// `char_idx` clamps to `s.len()`.
///
/// Needed at the boundary between `livemd` (byte-offset-based, per the
/// contract) and the rest of the editor, where `Cursor::column` and egui's
/// `CCursor` are **character** indices, not byte offsets.
pub fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
}

/// Converts a byte offset within `s` to a char index. Total: a `byte_idx`
/// past the end (or off a char boundary) clamps to the count of chars whose
/// start byte is `< byte_idx`.
pub fn byte_to_char(s: &str, byte_idx: usize) -> usize {
    s.char_indices().take_while(|&(b, _)| b < byte_idx).count()
}

#[cfg(test)]
mod byte_char_tests {
    use super::*;

    #[test]
    fn ascii_round_trip() {
        let s = "hello world";
        for c in 0..=s.chars().count() {
            let b = char_to_byte(s, c);
            assert_eq!(byte_to_char(s, b), c);
        }
    }

    #[test]
    fn cjk_and_emoji_round_trip() {
        let s = "日本語😀end";
        let n = s.chars().count();
        for c in 0..=n {
            let b = char_to_byte(s, c);
            assert!(s.is_char_boundary(b));
            assert_eq!(byte_to_char(s, b), c);
        }
    }

    #[test]
    fn out_of_range_clamps() {
        let s = "abc";
        assert_eq!(char_to_byte(s, 9999), s.len());
        assert_eq!(byte_to_char(s, 9999), s.chars().count());
    }

    #[test]
    fn empty_string() {
        assert_eq!(char_to_byte("", 0), 0);
        assert_eq!(byte_to_char("", 0), 0);
    }
}

/// Whether a span is content the user reads, or syntax that can be hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanRole {
    Text,
    Marker,
}

/// Byte range within the SOURCE line, plus how to draw it.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleSpan {
    /// Byte offsets into the source line (never char offsets).
    pub range: std::ops::Range<usize>,
    pub style: InlineStyle,
    pub role: SpanRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub link: bool,
    pub heading: Option<u8>,
    pub blockquote: bool,
}
