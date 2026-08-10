//! Text rendering for FerriteEditor.

use egui::{Color32, FontFamily, FontId, Pos2, Rect};
use std::sync::Arc;

use super::super::line_cache::LineCache;

/// Renders a line of text using the cache.
pub fn render_line(
    painter: &egui::Painter,
    line_cache: &mut LineCache,
    line_content: &str,
    x: f32,
    y: f32,
    font_id: FontId,
    text_color: Color32,
) -> Arc<egui::Galley> {
    // Strip trailing newline for display
    let display_content = line_content.trim_end_matches(['\r', '\n']);

    // Get or create galley from cache
    let galley = line_cache.get_galley(display_content, painter, font_id, text_color);

    // Draw the galley
    painter.galley(Pos2::new(x, y), Arc::clone(&galley), text_color);

    galley
}

/// Paints inline-code background chips directly from a galley's glyph
/// positions, in place of `TextFormat.background` (see
/// `livemd::style::text_format_for` for why: that ties the box to the
/// inline-code span's `line_height`, which is deliberately inflated to pull
/// the mono baseline onto the prose baseline, and the background box
/// inherited the inflation — overshooting below with no top padding).
///
/// Call this immediately **before** painting `galley` at `origin`, with the
/// same `origin` the caller is about to pass to `painter.galley(...)`. Not
/// for fenced-code lines: those get a full-width band painted separately and
/// must not also get per-span chips (see `FerriteEditor::ui`).
///
/// `Glyph::section_index` is private to epaint, so a run's section is instead
/// tracked by hand: `job.sections` are contiguous, ordered byte ranges into
/// `job.text`, and every galley built for a line here has none of its own
/// (rows are word-wrap breaks, not paragraph breaks), so a running byte
/// cursor advanced by each glyph's UTF-8 length stays in step with the
/// section boundaries.
pub fn paint_inline_code_chips(
    painter: &egui::Painter,
    origin: Pos2,
    galley: &egui::Galley,
    chip_color: Color32,
) {
    let job = &galley.job;
    let code_family = FontFamily::Name(crate::fonts::FONT_JETBRAINS.into());

    let mut byte_cursor = 0usize;
    let mut section_cursor = 0usize;

    for placed_row in &galley.rows {
        // (x0, x1, baseline_y, code_size) of the run in progress, all
        // relative to the row.
        let mut run: Option<(f32, f32, f32, f32)> = None;

        for glyph in &placed_row.row.glyphs {
            while section_cursor + 1 < job.sections.len()
                && byte_cursor >= job.sections[section_cursor].byte_range.end
            {
                section_cursor += 1;
            }
            let section = job.sections.get(section_cursor);
            let is_code = section.is_some_and(|s| s.format.font_id.family == code_family);

            if is_code {
                let code_size = section.map_or(glyph.font_height, |s| s.format.font_id.size);
                let x1 = glyph.pos.x + glyph.advance_width;
                run = Some(match run {
                    Some((x0, _, baseline_y, _)) => (x0, x1, baseline_y, code_size),
                    None => (glyph.pos.x, x1, glyph.pos.y, code_size),
                });
            } else if let Some(r) = run.take() {
                paint_chip(painter, origin, placed_row.pos, r, chip_color);
            }

            byte_cursor += glyph.chr.len_utf8();
        }

        if let Some(r) = run.take() {
            paint_chip(painter, origin, placed_row.pos, r, chip_color);
        }
    }
}

/// Paints one inline-code chip. `(x0, x1, baseline_y, code_size)` are all
/// relative to the row; `origin + row_pos` places them in painter space.
///
/// `CAP_EM_JETBRAINS`/`DESC_EM_JETBRAINS` are outline-measured, not the
/// `hhea` ascent/descent — see `fonts::CAP_EM_JETBRAINS`. Rounded by `pad`
/// for a chip rather than a hard-edged box.
fn paint_chip(
    painter: &egui::Painter,
    origin: Pos2,
    row_pos: Pos2,
    (x0, x1, baseline_y, code_size): (f32, f32, f32, f32),
    chip_color: Color32,
) {
    let pad = 0.12 * code_size;
    let y_top = baseline_y - crate::fonts::CAP_EM_JETBRAINS * code_size - pad;
    let y_bot = baseline_y + crate::fonts::DESC_EM_JETBRAINS * code_size + pad;
    let rect = Rect::from_min_max(
        Pos2::new(origin.x + row_pos.x + x0 - pad, origin.y + row_pos.y + y_top),
        Pos2::new(origin.x + row_pos.x + x1 + pad, origin.y + row_pos.y + y_bot),
    );
    painter.rect_filled(rect, pad, chip_color);
}
