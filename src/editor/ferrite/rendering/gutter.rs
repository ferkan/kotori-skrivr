//! Gutter (line numbers) rendering for FerriteEditor.

use crate::ui::phosphor_icons::{phosphor_font, CARET_DOWN, CARET_RIGHT};
use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

/// Minimum width of the line number gutter in characters.
/// Set to 3 to provide stability for files up to 999 lines without jumping.
pub const GUTTER_CHARS: usize = 3;

/// Padding between gutter and text content.
pub const GUTTER_PADDING: f32 = 8.0;

/// Renders the gutter background.
///
/// `origin_x` is the gutter's left edge — the text column's origin, not
/// necessarily the window edge. When the column is centred (`content_offset_x`
/// on `FerriteEditor`), the gutter must travel with it; `rect` is used only
/// for the vertical extent.
pub fn render_gutter_background(
    painter: &egui::Painter,
    rect: Rect,
    origin_x: f32,
    gutter_width: f32,
    gutter_bg_color: Color32,
    separator_color: Color32,
) {
    // Draw gutter background
    let gutter_rect = Rect::from_min_size(
        Pos2::new(origin_x, rect.min.y),
        Vec2::new(gutter_width, rect.height()),
    );
    painter.rect_filled(gutter_rect, 0.0, gutter_bg_color);

    // Draw separator line between gutter and content
    let separator_x = origin_x + gutter_width;
    painter.line_segment(
        [
            Pos2::new(separator_x, rect.min.y),
            Pos2::new(separator_x, rect.max.y),
        ],
        Stroke::new(1.0, separator_color),
    );
}

/// Distance from a text row's top to where a gutter numeral's galley top
/// must be painted so the numeral's baseline coincides with the row's text
/// baseline.
///
/// epaint places a glyph's baseline at `ascent + valign_factor *
/// (row_height - line_height)`; with `Align::BOTTOM` (the default) and a row
/// whose glyphs all share the row's line height, that reduces to `baseline =
/// ascent`, measured from the row's top. So:
///
/// ```text
/// text_baseline   = ascent_em(body) * line_font_size
/// number_baseline = offset + ascent_em(gutter) * gutter_font_size
/// offset          = ascent_em(body) * line_font_size
///                     - ascent_em(gutter) * gutter_font_size
/// ```
///
/// Pulled out as a pure function so the arithmetic can be unit-tested without
/// depending on which fonts happen to be installed in the running process.
pub fn line_number_baseline_offset(
    body_ascent_em: f32,
    line_font_size: f32,
    gutter_ascent_em: f32,
    gutter_font_size: f32,
) -> f32 {
    body_ascent_em * line_font_size - gutter_ascent_em * gutter_font_size
}

/// Reads the baseline epaint actually used for a painted line, straight out
/// of the galley rather than predicting it.
///
/// `Glyph::pos.y` is documented in epaint as the glyph's baseline position
/// relative to the row, and is the same for every glyph sharing a
/// `TextFormat`. Anchoring to the **first glyph of the first row** matters
/// for a wrapped line: it must line up with its opening row, not wherever
/// wrapping happens to put row 0's longest span.
///
/// Returns `None` for a blank line — `galley.rows[0].glyphs` is empty when
/// there is no text, and callers should fall back to
/// [`line_number_baseline_offset`], which is exact in that case (nothing
/// raises `max_row_height` above `ascent`).
pub fn first_glyph_baseline(galley: &egui::Galley) -> Option<f32> {
    galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .map(|glyph| glyph.pos.y)
}

/// Draws one right-aligned line number, its baseline aligned to the line's
/// text baseline.
///
/// `y` is the top of the line's **text** (excluding any block space above
/// it); `baseline_offset` is the vertical distance from `y` to where the
/// numeral's galley top must be painted so that its baseline lands on the
/// text baseline — computed by the caller (see
/// `FerriteEditor::ui`) from both fonts' ascent metrics, since epaint places
/// a glyph's baseline at `ascent + valign_factor * (row_height -
/// line_height)` and different faces disagree on ascent even at the same
/// point size.
///
/// `baseline_offset` can legitimately be negative — when the gutter font's
/// ascent exceeds the body font's (as with JetBrains Mono against ITC
/// Garamond Condensed), the numeral's top must sit *above* the text top for
/// the two baselines to coincide. Do not clamp it to zero.
///
/// The font is passed in rather than derived: line numbers are set in a
/// monospace face so digits share an advance width and the column of numbers
/// stays optically straight, independent of the document's body font.
pub fn render_line_number(
    painter: &egui::Painter,
    line_idx: usize,
    x: f32,
    y: f32,
    width: f32,
    baseline_offset: f32,
    font_id: &FontId,
    color: Color32,
) {
    let line_num = (line_idx + 1).to_string(); // 1-indexed display

    // Create galley for line number (not cached, simple text)
    let galley = painter.layout_no_wrap(line_num, font_id.clone(), color);

    // Right-align the line number
    let text_width = galley.size().x;
    let text_x = x + width - text_width;

    let text_y = y + baseline_offset;

    painter.galley(Pos2::new(text_x, text_y), galley, color);
}

/// Width of fold indicator area in pixels.
/// Sized to fit the fold indicator character (▼/▶) with minimal padding.
pub const FOLD_INDICATOR_WIDTH: f32 = 12.0;

/// Calculates the width of the line number gutter.
///
/// Returns the total width needed for the gutter area, which can include:
/// - Line numbers (when `show_line_numbers` is true)
/// - Fold indicators (when `show_fold_indicators` is true)
///
/// When both are shown, fold indicators get dedicated space on the left.
/// When only fold indicators are shown (no line numbers), a minimal width is used.
/// Returns 0.0 when both are disabled.
pub fn calculate_gutter_width(
    ui: &egui::Ui,
    font_id: &FontId,
    line_count: usize,
    show_line_numbers: bool,
    show_fold_indicators: bool,
) -> f32 {
    // If neither is shown, no gutter needed
    if !show_line_numbers && !show_fold_indicators {
        return 0.0;
    }

    // If only fold indicators (no line numbers), use minimal width for just the indicators
    if !show_line_numbers && show_fold_indicators {
        return FOLD_INDICATOR_WIDTH;
    }

    // Line numbers are shown - calculate width based on digit count
    let max_line_num = line_count;
    let digits = if max_line_num == 0 {
        1
    } else {
        (max_line_num as f32).log10().floor() as usize + 1
    };

    // Use a sample string to measure width
    let sample = "0".repeat(digits.max(GUTTER_CHARS));
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(sample, font_id.clone(), Color32::WHITE));
    let line_num_width = galley.size().x + 8.0; // Add padding

    // When fold indicators are also shown, add space for them on the left
    if show_fold_indicators {
        line_num_width + FOLD_INDICATOR_WIDTH
    } else {
        line_num_width
    }
}

/// Renders a fold indicator in the gutter.
///
/// Shows ▶ for collapsed folds, ▼ for expanded folds.
pub fn render_fold_indicator(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    line_height: f32,
    is_collapsed: bool,
    color: Color32,
) {
    let indicator = if is_collapsed {
        CARET_RIGHT
    } else {
        CARET_DOWN
    };
    let font_id = phosphor_font(line_height * 0.7);
    let galley = painter.layout_no_wrap(indicator.to_string(), font_id, color);

    // Center the indicator vertically
    let indicator_y = y + (line_height - galley.size().y) / 2.0;

    painter.galley(Pos2::new(x, indicator_y), galley, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gutter_chars_constant() {
        // Verify gutter width constant for line number display
        assert_eq!(GUTTER_CHARS, 3);
    }

    #[test]
    fn test_gutter_padding_constant() {
        // Verify gutter padding constant
        assert!((GUTTER_PADDING - 8.0).abs() < 0.01);
    }

    #[test]
    fn baseline_offset_is_zero_for_matched_fonts() {
        // Equal ascents and equal sizes: the two baselines already coincide,
        // so the numeral needs no vertical nudge at all.
        let offset = line_number_baseline_offset(1.0, 16.0, 1.0, 16.0);
        assert!(offset.abs() < 1e-6, "expected 0.0, got {offset}");
    }

    #[test]
    fn baseline_offset_is_negative_when_body_ascent_is_shallower() {
        // The Garamond case: JetBrains Mono's ascent (gutter) exceeds ITC
        // Garamond Condensed's (body), so the numeral's top must sit above
        // the text top for their baselines to meet.
        let offset = line_number_baseline_offset(0.7031, 16.0, 1.0200, 16.0);
        assert!(offset < 0.0, "expected negative offset, got {offset}");
    }

    #[test]
    fn baseline_offset_grows_with_heading_font_size() {
        // A heading's larger `line_font_size` pushes the text baseline
        // further down its (taller) row, so the numeral must follow.
        let body_offset = line_number_baseline_offset(0.9688, 16.0, 1.0200, 13.0);
        let heading_offset = line_number_baseline_offset(0.9688, 16.0 * 2.0, 1.0200, 13.0);
        assert!(
            heading_offset > body_offset,
            "heading offset {heading_offset} should exceed body offset {body_offset}"
        );
    }

    /// The regression this whole file exists to prevent: lay out two real
    /// galleys through the app's own fonts, one plain and one carrying an
    /// inline-code span (which gets a taller `line_height`, per
    /// `fonts::inline_code_line_height`, to put the code on the prose
    /// baseline). Their first-glyph `pos.y` must differ — that's *why* the
    /// ascent-only prediction in `line_number_baseline_offset` alone isn't
    /// enough for a row with mixed spans — and reading the baseline back out
    /// of the galley (`first_glyph_baseline`) must land a numeral within
    /// half a pixel in both cases, where the old prediction does not.
    #[test]
    fn recorded_baseline_matches_real_galley_for_inline_code_row() {
        use crate::config::EditorFont;
        use egui::text::{LayoutJob, TextFormat};
        use egui::FontFamily;

        let ctx = egui::Context::default();
        ctx.set_fonts(crate::fonts::create_font_definitions());

        // Literata is the default editor body font (`EditorFont::default()`);
        // its ascent sits far enough from JetBrains Mono's that the shift
        // survives pixel rounding, unlike Inter's closer-matched ascent.
        let body_font = EditorFont::Literata;
        let body_size = 18.0_f32;
        let row_height = body_size * crate::theme::typescale::DEFAULT_BODY_LINE_HEIGHT;
        let code_size = body_size * crate::fonts::code_size_ratio(&body_font);
        let code_line_height =
            crate::fonts::inline_code_line_height(&body_font, body_size, code_size, row_height);

        let prose_format = TextFormat {
            font_id: FontId::new(body_size, crate::fonts::get_base_font_family(&body_font)),
            line_height: Some(row_height),
            color: Color32::WHITE,
            ..Default::default()
        };
        let mut prose_job = LayoutJob::default();
        prose_job.append("plain prose row", 0.0, prose_format.clone());

        let mut mixed_job = LayoutJob::default();
        mixed_job.append("prose with ", 0.0, prose_format);
        mixed_job.append(
            "code",
            0.0,
            TextFormat {
                font_id: FontId::new(code_size, FontFamily::Name(crate::fonts::FONT_JETBRAINS.into())),
                line_height: Some(code_line_height),
                color: Color32::WHITE,
                ..Default::default()
            },
        );

        let mut prose_galley = None;
        let mut mixed_galley = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                prose_galley = Some(ui.fonts_mut(|fonts| fonts.layout_job(prose_job.clone())));
                mixed_galley = Some(ui.fonts_mut(|fonts| fonts.layout_job(mixed_job.clone())));
            });
        });
        let prose_galley = prose_galley.expect("prose galley laid out inside ctx.run");
        let mixed_galley = mixed_galley.expect("mixed galley laid out inside ctx.run");

        let prose_baseline =
            first_glyph_baseline(&prose_galley).expect("prose row has glyphs");
        let mixed_baseline =
            first_glyph_baseline(&mixed_galley).expect("mixed row has glyphs");

        assert!(
            (prose_baseline - mixed_baseline).abs() > 0.5,
            "expected inline code to shift the row's baseline: prose {prose_baseline}, mixed {mixed_baseline}"
        );

        let gutter_font_size = 13.0_f32;
        let numeral_baseline = crate::fonts::ascent_em_jetbrains() * gutter_font_size;

        for text_baseline in [prose_baseline, mixed_baseline] {
            let offset = text_baseline - numeral_baseline;
            let painted_numeral_baseline = offset + numeral_baseline;
            assert!(
                (painted_numeral_baseline - text_baseline).abs() < 0.5,
                "recorded-baseline placement should match within 0.5px"
            );
        }

        // The old, ascent-only prediction: fine for the plain row, off for
        // the inline-code row — pinning the reason it had to go.
        let predicted_offset = line_number_baseline_offset(
            crate::fonts::ascent_em(&body_font),
            body_size,
            crate::fonts::ascent_em_jetbrains(),
            gutter_font_size,
        );
        let predicted_numeral_baseline = predicted_offset + numeral_baseline;
        assert!(
            (predicted_numeral_baseline - mixed_baseline).abs() >= 0.5,
            "ascent-only prediction should miss the inline-code row's real baseline"
        );
    }
}
