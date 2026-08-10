//! Mouse position conversion for FerriteEditor.
//!
//! This module contains:
//! - `pos_to_cursor` - Convert screen position to cursor position
//! - `calculate_column_from_pos` - Calculate column from click coordinates

use egui::{Color32, Pos2, Rect, Ui};

use super::cursor::Cursor;
use super::editor::FerriteEditor;

impl FerriteEditor {
    /// Calculates the column position from click coordinates.
    /// For wrapped text, uses both x and y_in_line to find the correct character.
    pub(crate) fn calculate_column_from_pos(
        &self,
        x: f32,
        y_in_line: f32,
        line: usize,
        font_id: &egui::FontId,
        wrap_width: f32,
        ui: &Ui,
    ) -> usize {
        if let Some(line_content) = self.buffer.get_line(line) {
            let line_content = line_content.trim_end_matches(['\r', '\n']);

            // Live inline markdown mode: the rendered text on screen (what
            // the user is actually clicking against) may not be the source
            // text at all -- hidden markers are omitted from the display
            // string, and revealed lines mix multiple font sizes/weights
            // (headings, bold, ...) that a single-FontId measurement below
            // cannot represent. Build the *actual* styled layout used for
            // rendering and hit-test against that instead, then map the
            // resulting **display** column back to a **source** column via
            // `LineMap::to_source` (identity on a revealed line). See
            // `.claude/skills/livemd/SKILL.md`, "The mapping rule".
            if self.live_markdown_enabled {
                return self.calculate_column_from_pos_livemd(
                    x,
                    y_in_line,
                    line,
                    line_content,
                    wrap_width,
                    ui,
                );
            }

            if self.wrap_enabled && wrap_width > 0.0 {
                // For wrapped text, create a wrapped galley and use cursor_from_pos
                let galley = ui.fonts_mut(|f| {
                    f.layout(
                        line_content.to_string(),
                        font_id.clone(),
                        Color32::WHITE,
                        wrap_width,
                    )
                });

                // cursor_from_pos takes a Vec2 position relative to the galley
                let pos = egui::vec2(x.max(0.0), y_in_line.max(0.0));
                let cursor = galley.cursor_from_pos(pos);
                cursor.index
            } else {
                // For non-wrapped text, use x-based calculation
                if x <= 0.0 {
                    return 0;
                }

                // Use HarfRust shaped advances for complex-script text
                if crate::fonts::needs_complex_script_fonts(line_content) {
                    let font_bytes = crate::fonts::ttf_bytes_for_font_id_shaping(font_id);
                    if let Some(col) = super::shaping::shaped_x_to_column(
                        line_content,
                        font_bytes,
                        font_id.size,
                        x,
                    ) {
                        return col.min(line_content.chars().count());
                    }
                }

                let chars: Vec<char> = line_content.chars().collect();
                let mut best_col = 0;
                let mut prev_width = 0.0;

                for (i, _) in chars.iter().enumerate() {
                    let prefix: String = chars[..=i].iter().collect();
                    let galley =
                        ui.fonts_mut(|f| f.layout_no_wrap(prefix, font_id.clone(), Color32::WHITE));
                    let width = galley.size().x;

                    let mid_point = (prev_width + width) / 2.0;
                    if x > mid_point {
                        best_col = i + 1;
                    }

                    prev_width = width;

                    if width > x {
                        break;
                    }
                }

                best_col.min(chars.len())
            }
        } else {
            0
        }
    }

    /// Live-inline-markdown variant of [`Self::calculate_column_from_pos`].
    ///
    /// Builds the same styled `LayoutJob` the render loop draws (via
    /// [`Self::livemd_styled_segments`]) and hit-tests against it with
    /// egui's own `cursor_from_pos`, rather than a single-`FontId`
    /// measurement -- required because live markdown mixes font sizes and
    /// weights within one line (headings, bold, ...), which a uniform-font
    /// measurement cannot represent.
    ///
    /// Returns a **source** char column: the display column egui resolves
    /// the click to is converted via `LineMap::to_source` (a no-op / not
    /// needed when the line is revealed, since its display text equals its
    /// source text; required when hidden, since marker text is then absent
    /// from what was measured).
    fn calculate_column_from_pos_livemd(
        &self,
        x: f32,
        y_in_line: f32,
        line: usize,
        source_line: &str,
        wrap_width: f32,
        ui: &Ui,
    ) -> usize {
        use super::livemd::{byte_to_char, char_to_byte};

        // Color is never painted here -- only used to build a valid
        // TextFormat for width measurement.
        let (revealed, segments, _) =
            self.livemd_styled_segments(line, source_line, Color32::WHITE, Color32::TRANSPARENT);

        let display_text: String = segments.iter().map(|s| s.text.as_str()).collect();

        let effective_wrap_width = if self.wrap_enabled && wrap_width > 0.0 {
            wrap_width
        } else {
            f32::INFINITY
        };

        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = effective_wrap_width;
        if segments.is_empty() {
            job.append("", 0.0, egui::text::TextFormat::default());
        }
        for seg in &segments {
            job.append(&seg.text, 0.0, seg.format.clone());
        }
        let galley = ui.fonts_mut(|f| f.layout_job(job));

        let pos = egui::vec2(x.max(0.0), y_in_line.max(0.0));
        let disp_char = galley.cursor_from_pos(pos).index;
        let disp_byte = char_to_byte(&display_text, disp_char);

        let src_byte = if revealed {
            // Revealed: LineMap is identity, and `display_text` is exactly
            // `source_line` char-for-char (markers are present, just
            // dimmed) -- no mapping needed.
            disp_byte
        } else {
            let map = self.livemd_line_map(line, source_line);
            map.to_source(disp_byte)
        };

        byte_to_char(source_line, src_byte)
    }

    /// Converts a y-coordinate to a line number.
    /// Used for fold indicator click detection.
    ///
    /// This function accounts for folded (hidden) lines - when lines are collapsed,
    /// the visual y-position doesn't map 1-to-1 with document lines.
    pub(crate) fn y_to_line(&self, y: f32, rect_min_y: f32, total_lines: usize) -> usize {
        let relative_y = y - rect_min_y;
        let first_visible = self.view.first_visible_line();
        let mut y_acc = -self.view.scroll_offset_y();

        // Iterate through lines, skipping hidden lines (same as rendering)
        for line_idx in first_visible..total_lines {
            // Skip lines hidden by collapsed folds
            if self.fold_state.is_line_hidden(line_idx) {
                continue;
            }

            let line_height = self.view.get_line_height(line_idx);
            if relative_y < y_acc + line_height {
                return line_idx;
            }
            y_acc += line_height;
        }

        // Past the last visible line - find the last non-hidden line
        for line_idx in (0..total_lines).rev() {
            if !self.fold_state.is_line_hidden(line_idx) {
                return line_idx;
            }
        }

        // Fallback (shouldn't happen unless document is empty)
        total_lines.saturating_sub(1)
    }

    /// Converts a screen position to a cursor position.
    ///
    /// This function accounts for folded (hidden) lines - when lines are collapsed,
    /// the visual y-position doesn't map 1-to-1 with document lines.
    pub(crate) fn pos_to_cursor(
        &self,
        pos: Pos2,
        rect: Rect,
        text_start_x: f32,
        font_id: &egui::FontId,
        wrap_width: f32,
        total_lines: usize,
        ui: &Ui,
    ) -> Cursor {
        let relative_y = pos.y - rect.min.y;
        let first_visible = self.view.first_visible_line();

        // Calculate clicked line and y position within that line
        // Both wrapped and non-wrapped modes need to account for folded lines
        let (clicked_line, y_in_line) = {
            // Start y_acc at -scroll_offset_y to match rendering which places first_visible_line
            // at rect.min.y - scroll_offset_y (i.e., scroll_offset_y pixels ABOVE rect.min.y)
            let mut y_acc = -self.view.scroll_offset_y();
            let mut result_line = first_visible;
            let mut result_y_in_line = 0.0;

            for line_idx in first_visible..total_lines {
                // Skip lines hidden by collapsed folds (same as rendering)
                if self.fold_state.is_line_hidden(line_idx) {
                    continue;
                }

                // Use the same per-line heights the render loop positions
                // lines with (`ViewState::get_line_height`, fed by
                // `set_line_wrap_info` from the galley actually drawn).
                //
                // Measuring here independently is what caused clicks to land
                // several lines away in live inline markdown: that mode draws
                // headings at up to 1.8x the body size, but a re-layout with
                // the plain `font_id` measures every line at 1.0x. The error
                // is per-heading and accumulates down the document. Deferring
                // to the view keeps hit-testing and rendering consistent by
                // construction, and matches what `y_to_line` already does.
                let line_height = self.view.get_line_height(line_idx);

                if relative_y < y_acc + line_height {
                    result_line = line_idx;
                    // `y_acc` is the ROW top (it accumulates the full
                    // `get_line_height`, including any blank space above a
                    // heading), but the galley -- and thus the column
                    // measurement below -- starts at the TEXT top, i.e.
                    // `y_acc + space_above` (see `line_y_positions` in the
                    // render loop, which stores the same text top). A click
                    // landing in that blank space must resolve to the start
                    // of the heading's text, not a negative offset into it.
                    let space_above = self.view.get_line_space_above(line_idx);
                    result_y_in_line = (relative_y - y_acc - space_above).max(0.0);
                    break;
                }
                y_acc += line_height;
                result_line = line_idx;
            }

            (result_line, result_y_in_line)
        };

        let clicked_line = clicked_line.min(total_lines.saturating_sub(1));
        let relative_x = pos.x - text_start_x + self.view.horizontal_scroll();
        let clicked_col = self.calculate_column_from_pos(
            relative_x,
            y_in_line,
            clicked_line,
            font_id,
            wrap_width,
            ui,
        );

        Cursor::new(clicked_line, clicked_col)
    }
}
