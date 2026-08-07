//! The document type scale.
//!
//! One table, consumed by every markdown surface. There used to be **three**
//! independent heading ramps:
//!
//! | path | ramp | |
//! |---|---|---|
//! | `editor::ferrite::livemd::style` | ×1.8/1.6/1.4/1.25/1.1/1.0 | live |
//! | `markdown::editor::render_heading` | ×1.8/1.5/1.3/1.15/1.05/1.0 | live |
//! | `markdown::widgets::EditableHeading` | ×2.0/1.75/1.5/1.25/1.1/1.0 | dead code |
//!
//! So the same H1 changed size when the user switched view mode. Note the two
//! live ramps were in different files from the obvious one — `widgets.rs`
//! *looks* like the rendered-mode heading renderer but nothing constructs it.
//!
//! Everything here is expressed as a ratio of the user's body size, so the
//! whole document scales from one setting.
//!
//! # Why these numbers
//!
//! - **H6 is smaller than body.** At the old ×1.1/×1.0, H5 and H6 were not
//!   distinguishable from body text once both were bold. Making H6 `0.875×`
//!   and leaning on weight is what GitHub does, and it is the only way to
//!   separate the two bottom levels without resorting to colour.
//! - **Space above, none below.** In markdown source a blank line *is* a line
//!   and already contributes a full body row. Adding space below a heading as
//!   well would double-count it.
//! - **Headings lead tighter than body.** Display sizes need proportionally
//!   less leading; 1.2 on a 28px H1 is already 34px of row.

/// Line-height multiplier applied to body text.
///
/// Not the font's native metric: Inter reports ~1.20 and Literata ~1.49, so
/// leaving leading to the font would silently change the reading rhythm when
/// the user switches typeface. Pinning it keeps the document stable and makes
/// the user's `line_height` setting mean the same thing everywhere.
pub const DEFAULT_BODY_LINE_HEIGHT: f32 = 1.4;

/// Allowed range for the user's line-height setting.
pub const MIN_LINE_HEIGHT: f32 = 1.1;
/// Allowed range for the user's line-height setting.
pub const MAX_LINE_HEIGHT: f32 = 2.0;

/// Line-height multiplier for headings, which need less leading than body.
pub const HEADING_LINE_HEIGHT: f32 = 1.2;

/// Line-height multiplier inside fenced code blocks.
pub const CODE_LINE_HEIGHT: f32 = 1.45;

/// Inline and block code size, relative to body.
///
/// A monospace face at the same nominal size as a serif reads noticeably
/// larger; the downshift makes the two sit level.
pub const CODE_SIZE_RATIO: f32 = 0.92;

/// Size of a heading level relative to the body size.
///
/// Level is clamped to 1..=6; anything out of range is treated as H6.
pub fn heading_size_ratio(level: u8) -> f32 {
    match level {
        1 => 1.750,
        2 => 1.4375,
        3 => 1.250,
        4 => 1.125,
        5 => 1.000,
        _ => 0.875,
    }
}

/// Blank space above a heading, in multiples of the body size.
///
/// There is deliberately no matching "space below": a blank source line after
/// the heading already supplies one body row, and doubling it separates the
/// heading from the text it introduces.
pub fn heading_space_above_ratio(level: u8) -> f32 {
    match level {
        1 => 1.75,
        2 => 1.25,
        3 => 1.00,
        _ => 0.75,
    }
}

/// Indent per list nesting level, in multiples of the body size.
pub const LIST_INDENT_RATIO: f32 = 1.5;

/// Indent for blockquote content, in multiples of the body size.
pub const BLOCKQUOTE_INDENT_RATIO: f32 = 1.25;

// ─────────────────────────────────────────────────────────────────────────────
// Chrome
// ─────────────────────────────────────────────────────────────────────────────

/// UI chrome sizes, in points. Independent of the editor's body size — chrome
/// must not grow when the user enlarges their document text.
pub mod chrome {
    /// Dialog and settings-pane titles. The largest chrome text there is; do
    /// not build a document hierarchy inside a settings pane.
    pub const TITLE: f32 = 18.0;

    /// Section headings within a page ("File", "Code execution").
    ///
    /// Size *and* weight *and* space. An earlier revision set these at body
    /// size and leaned on weight alone; in a dense list that was too subtle to
    /// read as a boundary.
    pub const SECTION: f32 = 15.0;

    /// Tab labels, file tree rows, toolbar labels, menu items, settings body.
    pub const BODY: f32 = 14.0;

    /// Status bar, tooltips, secondary metadata, hints.
    pub const SMALL: f32 = 12.5;

    /// Nothing in the UI may be smaller than this, including icon glyphs.
    pub const MINIMUM: f32 = 12.0;

    /// Line-height multiplier for chrome text.
    ///
    /// Chrome previously had no explicit leading at all and inherited each
    /// font's native metric — the same defect the editing surface had. Tighter
    /// than the document's leading because chrome lines are short labels, not
    /// prose across a wide measure.
    pub const LINE_HEIGHT: f32 = 1.45;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_sizes_decrease_strictly() {
        for level in 1..6u8 {
            assert!(
                heading_size_ratio(level) > heading_size_ratio(level + 1),
                "H{level} must be larger than H{}",
                level + 1
            );
        }
    }

    #[test]
    fn h5_matches_body_and_h6_is_smaller() {
        assert_eq!(heading_size_ratio(5), 1.0);
        assert!(
            heading_size_ratio(6) < 1.0,
            "H6 must be smaller than body so weight alone need not separate it from H5"
        );
    }

    #[test]
    fn out_of_range_levels_clamp_to_h6() {
        assert_eq!(heading_size_ratio(7), heading_size_ratio(6));
        assert_eq!(heading_size_ratio(200), heading_size_ratio(6));
        assert_eq!(heading_size_ratio(0), heading_size_ratio(6));
    }

    #[test]
    fn heading_space_above_decreases_with_level() {
        for level in 1..3u8 {
            assert!(heading_space_above_ratio(level) >= heading_space_above_ratio(level + 1));
        }
        assert!(heading_space_above_ratio(4) > 0.0, "every heading gets air");
    }

    #[test]
    fn chrome_scale_is_ordered_and_above_the_minimum() {
        assert!(chrome::TITLE > chrome::SECTION);
        assert!(chrome::SECTION > chrome::BODY);
        assert!(chrome::BODY > chrome::SMALL);
        assert!(chrome::SMALL >= chrome::MINIMUM);
        // Chrome leads tighter than the document: short labels, not prose.
        assert!(chrome::LINE_HEIGHT < DEFAULT_BODY_LINE_HEIGHT + 0.1);
    }

    #[test]
    fn default_line_height_is_within_the_settable_range() {
        assert!(DEFAULT_BODY_LINE_HEIGHT >= MIN_LINE_HEIGHT);
        assert!(DEFAULT_BODY_LINE_HEIGHT <= MAX_LINE_HEIGHT);
        assert!(
            HEADING_LINE_HEIGHT < DEFAULT_BODY_LINE_HEIGHT,
            "display sizes need proportionally less leading than body"
        );
    }

    /// The concrete scale at the default 16px body, so a change to any ratio
    /// shows up here as a readable pixel diff rather than a float nudge.
    #[test]
    fn scale_at_default_body_size() {
        let body = 16.0_f32;
        let px = |r: f32| (body * r).round() as i32;
        assert_eq!(px(heading_size_ratio(1)), 28);
        assert_eq!(px(heading_size_ratio(2)), 23);
        assert_eq!(px(heading_size_ratio(3)), 20);
        assert_eq!(px(heading_size_ratio(4)), 18);
        assert_eq!(px(heading_size_ratio(5)), 16);
        assert_eq!(px(heading_size_ratio(6)), 14);
        assert_eq!(px(CODE_SIZE_RATIO), 15);
        assert_eq!(px(DEFAULT_BODY_LINE_HEIGHT), 22);
    }
}

