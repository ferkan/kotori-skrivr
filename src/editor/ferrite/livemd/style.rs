//! `InlineStyle` -> egui `TextFormat` mapping for the live inline markdown mode.
//!
//! This is the one file under `livemd/` that touches egui types (see the
//! module layout table in `.claude/skills/livemd/SKILL.md`). Font family
//! selection reuses `crate::fonts::styled_font_id`, which already maps
//! (bold, italic, `EditorFont`) to the correct registered font family
//! (`fonts.rs:917-937`) — this module does not duplicate that table, it only
//! adds heading size scaling and marker dimming on top of it.

use egui::text::TextFormat;
use egui::{Color32, Stroke};

use super::InlineStyle;
use crate::config::EditorFont;
use crate::fonts::styled_font_id;
use crate::theme::typescale;

/// Alpha multiplier applied to the body text color when rendering a revealed
/// marker span, so markers read as dimmed syntax rather than content.
///
/// Markers are text the user reads and edits once the cursor reveals a line —
/// a link's `](https://…)` is not decoration. At 0.35 they measured ~2.6:1 in
/// the dark theme, below the readable floor; 0.55 lands near 4.5:1 while still
/// clearly receding behind the content.
const MARKER_ALPHA_SCALE: f32 = 0.55;

/// Resolves the font size for a span given the document's base body size.
///
/// Heading sizing comes from `typescale::heading_size_ratio`, the single
/// ramp shared with Rendered mode (`markdown::widgets`) — this used to be a
/// locally-owned ×1.8..1.0 ramp, so the same H1 changed size when the user
/// switched view mode.
fn resolve_size(style: &InlineStyle, base_size: f32, editor_font: &EditorFont) -> f32 {
    match style.heading {
        Some(level) => base_size * typescale::heading_size_ratio(level),
        None if style.code => base_size * crate::fonts::code_size_ratio(editor_font),
        None => base_size,
    }
}

/// Whether a span renders with the bold font family.
///
/// Heading text is bold in addition to being larger. `scan.rs` deliberately
/// does not set `bold` on heading spans — it records the *structure* it parsed
/// (`heading: Some(level)`), leaving how that structure looks to this layer.
/// Size alone reads as an oversized paragraph rather than a heading.
fn is_bold(style: &InlineStyle) -> bool {
    style.bold || style.heading.is_some()
}

/// Builds the `TextFormat` for a **content** (`SpanRole::Text`) span.
///
/// * Inline `code` always uses the JetBrains family, regardless of the
///   editor's configured font — this is a hard rule from the contract, not a
///   default that can be overridden by `editor_font`.
/// * Bold/italic select the matching pre-registered font family variant via
///   `styled_font_id` rather than emulating weight/slant with color.
/// * Strikethrough is expressed with egui's native strikethrough stroke.
/// * `line_height_px` is decided once per LINE by the caller, not derived
///   from this span's own style — inline code sharing a line with prose must
///   report the same leading as the prose, or egui's per-row max-height
///   layout puts them on different baselines (this was a real, user-visible
///   bug: see `livemd_styled_segments` in `editor.rs`).
/// * `code_bg` is accepted for interface symmetry with `marker_format_for`
///   but is **not** applied here: `TextFormat.background` paints over
///   `Glyph::logical_rect()`, whose height is the span's `line_height` —
///   deliberately inflated for inline code (see `fonts::inline_code_line_height`)
///   to pull the mono baseline onto the prose baseline. The background box
///   inherited that inflation, overshooting below with no top padding. Inline
///   code backgrounds are instead painted directly from glyph positions,
///   immediately before the galley — see
///   `rendering::text::paint_inline_code_chips`.
pub fn text_format_for(
    style: &InlineStyle,
    editor_font: &EditorFont,
    base_size: f32,
    body_color: Color32,
    _code_bg: Color32,
    line_height_px: f32,
    in_fence: bool,
) -> TextFormat {
    let size = resolve_size(style, base_size, editor_font);
    let font_for_family = if style.code {
        &EditorFont::JetBrainsMono
    } else {
        editor_font
    };
    let font_id = styled_font_id(size, is_bold(style), style.italic, font_for_family);

    // A code span on a prose line needs its baseline pulled down onto the
    // prose baseline. Equal `line_height` alone is not enough: epaint places a
    // baseline at `ascent + valign_factor * (row_height - line_height)`, so
    // when both spans report the same line height that term vanishes and each
    // lands at its own font's ascent. JetBrains Mono's ascent is much shallower
    // than Literata's, which left inline code floating ~3.8 px high at a 16 px
    // body. Solving for the code span's line height puts them back in line.
    //
    // Only for code *within* prose — on a fenced line every span is code, so
    // they already share an ascent and the correction would skew the block.
    let effective_line_height = if style.code && style.heading.is_none() && !in_fence {
        crate::fonts::inline_code_line_height(editor_font, base_size, size, line_height_px)
    } else {
        line_height_px
    };

    let mut format = TextFormat {
        font_id,
        color: body_color,
        line_height: Some(effective_line_height),
        ..Default::default()
    };

    if style.strikethrough {
        format.strikethrough = Stroke::new(1.0, body_color);
    }

    format
}

/// Builds the `TextFormat` for a **marker** (`SpanRole::Marker`) span that is
/// currently revealed (the line's cursor/selection makes it visible). Markers
/// use the same font/size as the surrounding content but at reduced alpha, so
/// they read as syntax rather than content — never emulated with a hue shift.
/// `line_height_px` follows the same per-line rule as `text_format_for`.
pub fn marker_format_for(
    style: &InlineStyle,
    editor_font: &EditorFont,
    base_size: f32,
    body_color: Color32,
    code_bg: Color32,
    line_height_px: f32,
    in_fence: bool,
) -> TextFormat {
    let mut format = text_format_for(
        style,
        editor_font,
        base_size,
        body_color,
        code_bg,
        line_height_px,
        in_fence,
    );
    format.color = dim(format.color);
    if style.strikethrough {
        format.strikethrough = Stroke::new(1.0, format.color);
    }
    format
}

/// Reduces a color's alpha to `MARKER_ALPHA_SCALE` of its original value.
fn dim(color: Color32) -> Color32 {
    let a = (color.a() as f32 * MARKER_ALPHA_SCALE).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_SIZE: f32 = 14.0;
    const BODY: Color32 = Color32::from_rgb(20, 20, 20);
    const CODE_BG: Color32 = Color32::from_rgb(40, 40, 40);
    /// Arbitrary line height used by tests that don't care about its value,
    /// only that `line_height_px` is passed through verbatim (see
    /// `line_height_is_set_verbatim_from_caller` below for the case that does).
    const LINE_HEIGHT: f32 = 20.0;

    #[test]
    fn heading_text_is_bold_and_larger_than_body() {
        let body = InlineStyle::default();
        let h1 = InlineStyle {
            heading: Some(1),
            ..Default::default()
        };
        let body_fmt = text_format_for(&body, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        let h1_fmt = text_format_for(&h1, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);

        assert!(
            h1_fmt.font_id.size > body_fmt.font_id.size,
            "heading must be larger than body text"
        );
        let bold_body = InlineStyle {
            bold: true,
            ..Default::default()
        };
        let bold_fmt = text_format_for(&bold_body, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert_eq!(
            h1_fmt.font_id.family, bold_fmt.font_id.family,
            "heading must use the same bold family as explicitly bold text, \
             not the regular family"
        );
    }

    /// Weight must come from a real registered bold family, never from a
    /// color/hue change — the contract forbids emulating weight with color.
    #[test]
    fn heading_does_not_alter_color() {
        let h2 = InlineStyle {
            heading: Some(2),
            ..Default::default()
        };
        let fmt = text_format_for(&h2, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert_eq!(fmt.color, BODY);
    }

    /// Inline code no longer carries a `TextFormat.background` — the chip is
    /// painted separately from glyph positions (see
    /// `rendering::text::paint_inline_code_chips`), because tying it to
    /// `line_height` (deliberately inflated for baseline alignment) produced
    /// a box that overshot below with no top padding.
    #[test]
    fn inline_code_carries_no_text_format_background() {
        let prose = InlineStyle::default();
        let code = InlineStyle {
            code: true,
            ..Default::default()
        };
        let prose_fmt = text_format_for(&prose, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        let code_fmt = text_format_for(&code, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert_eq!(prose_fmt.background, Color32::TRANSPARENT);
        assert_eq!(code_fmt.background, Color32::TRANSPARENT);
    }

    /// A fenced line is entirely code and gets its own full-width band
    /// painted separately, so a per-span background here would double up
    /// (and stop ragged at the text's right edge) — moot now that inline
    /// code never sets `background`, but kept as an explicit regression pin.
    #[test]
    fn code_in_a_fence_gets_no_per_span_background() {
        let code = InlineStyle {
            code: true,
            ..Default::default()
        };
        let fmt = text_format_for(&code, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, true);
        assert_eq!(fmt.background, Color32::TRANSPARENT);
    }

    #[test]
    fn inline_code_always_uses_jetbrains_regardless_of_editor_font() {
        let style = InlineStyle {
            code: true,
            ..Default::default()
        };
        let inter_format = text_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        let jb_format = text_format_for(&style, &EditorFont::JetBrainsMono, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert_eq!(inter_format.font_id.family, jb_format.font_id.family);
        assert!(matches!(
            &inter_format.font_id.family,
            egui::FontFamily::Name(n) if n.as_ref().starts_with("JetBrainsMono")
        ));
    }

    #[test]
    fn bold_and_italic_select_distinct_families() {
        let plain = InlineStyle::default();
        let bold = InlineStyle {
            bold: true,
            ..Default::default()
        };
        let italic = InlineStyle {
            italic: true,
            ..Default::default()
        };
        let bold_italic = InlineStyle {
            bold: true,
            italic: true,
            ..Default::default()
        };

        let f_plain = text_format_for(&plain, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false).font_id.family;
        let f_bold = text_format_for(&bold, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false).font_id.family;
        let f_italic = text_format_for(&italic, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false).font_id.family;
        let f_bi = text_format_for(&bold_italic, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false).font_id.family;

        assert_ne!(f_plain, f_bold);
        assert_ne!(f_plain, f_italic);
        assert_ne!(f_bold, f_bi);
        assert_ne!(f_italic, f_bi);
    }

    #[test]
    fn heading_sizes_scale_down_from_h1_to_h6() {
        let mut sizes = Vec::new();
        for level in 1..=6u8 {
            let style = InlineStyle {
                heading: Some(level),
                ..Default::default()
            };
            let fmt = text_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
            sizes.push(fmt.font_id.size);
        }
        for w in sizes.windows(2) {
            assert!(w[0] > w[1], "heading sizes should strictly decrease h1->h6: {sizes:?}");
        }
        // H6 should be close to (but need not exceed) body size.
        assert!(*sizes.last().unwrap() <= BASE_SIZE * 1.05);
        // H1 must be the largest, and clearly larger than body size.
        assert!(sizes[0] > BASE_SIZE);
    }

    #[test]
    fn non_heading_uses_base_size() {
        let style = InlineStyle::default();
        let fmt = text_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert_eq!(fmt.font_id.size, BASE_SIZE);
    }

    #[test]
    fn marker_format_is_dimmed_relative_to_body() {
        let style = InlineStyle::default();
        let content = text_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        let marker = marker_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert!(marker.color.a() < content.color.a());
        // `Color32` stores premultiplied alpha (see `ecolor::Color32` docs),
        // so raw `.r()/.g()/.b()` accessor bytes necessarily shrink along
        // with alpha even though the underlying hue is untouched -- comparing
        // those accessors directly against the undimmed color would be
        // comparing apples to premultiplied oranges. Instead, confirm `dim`
        // only changed the alpha channel by reconstructing what
        // `from_rgba_unmultiplied` produces for the *same* source (r, g, b)
        // at the marker's (reduced) alpha and expecting an exact match.
        let expected = Color32::from_rgba_unmultiplied(BODY.r(), BODY.g(), BODY.b(), marker.color.a());
        assert_eq!(marker.color, expected);
    }

    #[test]
    fn marker_format_preserves_font_and_size() {
        let style = InlineStyle {
            heading: Some(2),
            bold: true,
            ..Default::default()
        };
        let content = text_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        let marker = marker_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert_eq!(marker.font_id, content.font_id);
    }

    #[test]
    fn strikethrough_sets_stroke() {
        let style = InlineStyle {
            strikethrough: true,
            ..Default::default()
        };
        let fmt = text_format_for(&style, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, LINE_HEIGHT, false);
        assert!(fmt.strikethrough.width > 0.0);
    }

    /// Regression test for the inline-code misalignment bug: leading is now
    /// decided once per LINE by the caller and passed in verbatim, not
    /// re-derived per span from `style.code`/`style.heading`. A prose span
    /// and an inline-code span on the same line must report the identical
    /// `line_height`, or egui's per-row max-height layout puts them on
    /// different baselines.
    #[test]
    fn line_height_is_set_verbatim_from_caller_and_shared_across_span_kinds() {
        let prose = InlineStyle::default();
        let inline_code = InlineStyle {
            code: true,
            ..Default::default()
        };
        let line_height = BASE_SIZE * typescale::DEFAULT_BODY_LINE_HEIGHT;
        let prose_fmt = text_format_for(&prose, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, line_height, false);
        let code_fmt = text_format_for(&inline_code, &EditorFont::Inter, BASE_SIZE, BODY, CODE_BG, line_height, false);

        assert_eq!(prose_fmt.line_height, Some(line_height));
        // Inline code deliberately reports a DIFFERENT line_height — that is
        // the mechanism that aligns its baseline with the prose, not a bug.
        // See `baselines_align_between_prose_and_inline_code`.
        assert_ne!(
            prose_fmt.line_height, code_fmt.line_height,
            "inline code carries a baseline correction"
        );
    }
}

#[cfg(test)]
mod line_height_tests {
    use super::*;
    use crate::theme::typescale;

    const BASE: f32 = 16.0;
    const BODY: Color32 = Color32::BLACK;
    const CODE_BG: Color32 = Color32::from_rgb(40, 40, 40);

    fn style_with(code: bool, heading: Option<u8>) -> InlineStyle {
        InlineStyle {
            code,
            heading,
            ..Default::default()
        }
    }

    /// The regression this guards: leading is a property of the LINE, not the
    /// span. When inline code chose its own multiplier, a prose row containing
    /// `code` laid out at two different heights and egui put the code on a
    /// different baseline from the words either side of it.
    #[test]
    fn baselines_align_between_prose_and_inline_code() {
        let line_height = BASE * typescale::DEFAULT_BODY_LINE_HEIGHT;

        let prose = text_format_for(
            &style_with(false, None),
            &EditorFont::Literata,
            BASE,
            BODY,
            CODE_BG,
            line_height,
            false,
        );
        let code = text_format_for(
            &style_with(true, None),
            &EditorFont::Literata,
            BASE,
            BODY,
            CODE_BG,
            line_height,
            false,
        );

        // epaint places a baseline at
        //     ascent + valign_factor * (row_height - line_height)
        // with `Align::BOTTOM` (factor 1). Equal line heights would leave each
        // span at its own font's ascent — and JetBrains Mono's is far shallower
        // than Literata's, which is what left inline code floating ~3.8 px high.
        // So the correction lives in the code span's line_height.
        let row_height = line_height;
        let ascent_prose = crate::fonts::ascent_em(&EditorFont::Literata) * BASE;
        let ascent_code = crate::fonts::ascent_em(&EditorFont::JetBrainsMono) * code.font_id.size;

        let baseline_prose = ascent_prose;
        let baseline_code = ascent_code
            + (row_height - code.line_height.unwrap_or(row_height));

        assert!(
            (baseline_prose - baseline_code).abs() < 0.01,
            "baselines must coincide: prose {baseline_prose:.2} vs code {baseline_code:.2}"
        );

        // ...while still being a smaller monospace face.
        assert!(code.font_id.size < prose.font_id.size);
        assert_ne!(code.font_id.family, prose.font_id.family);
    }

    /// Markers sit on the same row as the text they annotate, so they inherit
    /// its leading too — they differ only in alpha.
    #[test]
    fn markers_share_the_line_leading() {
        let line_height = BASE * typescale::DEFAULT_BODY_LINE_HEIGHT;
        let text = text_format_for(
            &style_with(false, None),
            &EditorFont::Literata,
            BASE,
            BODY,
            CODE_BG,
            line_height,
            false,
        );
        let marker = marker_format_for(
            &style_with(false, None),
            &EditorFont::Literata,
            BASE,
            BODY,
            CODE_BG,
            line_height,
            false,
        );
        assert_eq!(text.line_height, marker.line_height);
        assert_ne!(text.color, marker.color, "markers are dimmed, not resized");
    }

    /// The caller decides the value; the style layer must apply it verbatim
    /// rather than re-deriving one per span.
    #[test]
    fn line_height_is_applied_verbatim() {
        for lh in [18.0_f32, 22.4, 40.0] {
            let fmt = text_format_for(
                &style_with(false, None),
                &EditorFont::Literata,
                BASE,
                BODY,
                CODE_BG,
                lh,
                false,
            );
            assert_eq!(fmt.line_height, Some(lh));
        }
    }
}
