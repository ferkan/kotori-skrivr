//! Skrivr editor icon mappings.
//!
//! Glyphs for the bundled `SkrivrIcons` font, built from the artwork in
//! `assets/icons/editor icons/` by the pipeline in `tools/iconfont/`. Central
//! place for editor icon codepoints so private-use characters don't get
//! scattered through the UI.
//!
//! Every glyph is fitted to an 860-unit box inside a 1000-unit em with a
//! uniform advance, so they sit evenly in square button slots without
//! per-icon nudging.
//!
//! The eight source icons with no markdown equivalent (text alignment,
//! underline, capitalize, columns, line height) are deliberately not in the
//! font — see `docs/technical/ui-review-2026-08.md`.

pub use crate::ui::icons::skrivr_font;

// ─────────────────────────────────────────────────────────────────────────────
// Inline formatting
// ─────────────────────────────────────────────────────────────────────────────

/// Bold (`**text**`).
pub const BOLD: &str = "\u{E001}";
/// Italic (`*text*`).
pub const ITALIC: &str = "\u{E002}";
/// Fenced code block.
pub const CODE_BLOCK: &str = "\u{E003}";
/// Blockquote.
pub const QUOTE: &str = "\u{E004}";
/// Insert link.
pub const LINK: &str = "\u{E005}";
/// Remove link.
///
/// Shipped in the font; no toolbar command exposes it yet.
#[allow(dead_code)]
pub const UNLINK: &str = "\u{E006}";

// ─────────────────────────────────────────────────────────────────────────────
// Lists and structure
// ─────────────────────────────────────────────────────────────────────────────

/// Unordered (bullet) list.
pub const LIST_UNORDERED: &str = "\u{E007}";
/// Ordered (numbered) list.
pub const LIST_NUMBERED: &str = "\u{E008}";
/// Lettered list.
///
/// Shipped in the font; no toolbar command exposes it yet.
#[allow(dead_code)]
pub const LIST_LETTERED: &str = "\u{E009}";

/// Outlined letter — document outline / table of contents.
pub const OUTLINE: &str = "\u{E00A}";
/// Initial letter (drop cap).
///
/// Present in the font but not currently wired to a command; markdown has no
/// drop-cap concept.
#[allow(dead_code)]
pub const INITIAL_LETTER: &str = "\u{E00B}";
/// Clear formatting.
///
/// Shipped in the font; no toolbar command exposes it yet.
#[allow(dead_code)]
pub const ERASE: &str = "\u{E00C}";

// ─────────────────────────────────────────────────────────────────────────────
// File and clipboard operations
//
// None of these are wired yet: the format toolbar is a formatting bar, and the
// ribbon still uses Phosphor. They are kept so the module stays a complete map
// of the font.
// ─────────────────────────────────────────────────────────────────────────────

/// Save document.
#[allow(dead_code)]
pub const SAVE: &str = "\u{E00D}";
/// Print document.
#[allow(dead_code)]
pub const PRINT: &str = "\u{E00E}";
/// Copy selection.
#[allow(dead_code)]
pub const COPY: &str = "\u{E00F}";
/// Cut selection.
#[allow(dead_code)]
pub const CUT: &str = "\u{E010}";
/// Paste from clipboard.
#[allow(dead_code)]
pub const PASTE: &str = "\u{E011}";

#[cfg(test)]
mod tests {
    use super::*;

    /// Every mapping the font actually ships, in codepoint order.
    const ALL: &[(&str, &str)] = &[
        ("BOLD", BOLD),
        ("ITALIC", ITALIC),
        ("CODE_BLOCK", CODE_BLOCK),
        ("QUOTE", QUOTE),
        ("LINK", LINK),
        ("UNLINK", UNLINK),
        ("LIST_UNORDERED", LIST_UNORDERED),
        ("LIST_NUMBERED", LIST_NUMBERED),
        ("LIST_LETTERED", LIST_LETTERED),
        ("OUTLINE", OUTLINE),
        ("INITIAL_LETTER", INITIAL_LETTER),
        ("ERASE", ERASE),
        ("SAVE", SAVE),
        ("PRINT", PRINT),
        ("COPY", COPY),
        ("CUT", CUT),
        ("PASTE", PASTE),
    ];

    #[test]
    fn glyphs_are_single_chars_in_the_font_range() {
        // The font defines U+E001..=U+E011; anything outside renders as tofu.
        for (name, glyph) in ALL {
            let mut chars = glyph.chars();
            let c = chars.next().unwrap_or('\0');
            assert!(chars.next().is_none(), "{name} must be a single char");
            assert!(
                ('\u{E001}'..='\u{E011}').contains(&c),
                "{name} is U+{:04X}, outside the font's range",
                c as u32
            );
        }
    }

    #[test]
    fn glyphs_are_distinct() {
        for (i, (name_a, a)) in ALL.iter().enumerate() {
            for (name_b, b) in &ALL[i + 1..] {
                assert_ne!(a, b, "{name_a} and {name_b} share a codepoint");
            }
        }
    }

    #[test]
    fn every_glyph_in_the_font_is_mapped() {
        assert_eq!(ALL.len(), 17, "font ships 17 glyphs; update the mappings");
    }
}
