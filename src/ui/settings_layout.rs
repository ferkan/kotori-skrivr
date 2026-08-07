//! Layout primitives for the settings panel.
//!
//! The settings pages had grown organically: section headings set at body size
//! and weight, dropdowns sized to whatever their widest entry happened to be,
//! description text at three different sizes, and long lists with no row
//! banding. Individually minor, together they read as an unfinished form.
//!
//! These helpers exist so a settings page composes from four things — a
//! section, a row, a description and a control — rather than from raw `ui`
//! calls that each make their own decisions. Sizes come from
//! [`crate::theme::typescale::chrome`] so chrome stays independent of the
//! user's document font size.

use crate::theme::typescale::chrome;
use eframe::egui::{self, Color32, RichText, Ui};

// ─────────────────────────────────────────────────────────────────────────────
// Metrics
// ─────────────────────────────────────────────────────────────────────────────

/// Width for every dropdown in settings.
///
/// Uniform by choice. Sizing each control to its own content made a column of
/// dropdowns look ragged, and the width then shifted when the selected value
/// changed — the control appearing to resize as you use it.
pub const CONTROL_WIDTH: f32 = 220.0;

/// Width for sliders, narrower than a dropdown so a slider and its numeric
/// readout together occupy about one control column.
pub const SLIDER_WIDTH: f32 = 150.0;

/// Vertical padding inside a settings row, above and below its content.
pub const ROW_PADDING_Y: f32 = 4.0;

/// Horizontal padding inside a settings row.
///
/// Banded rows need side padding or the text sits flush against the edge of
/// its own stripe.
pub const ROW_PADDING_X: f32 = 6.0;

/// Space above a section heading. Generous, because a heading's job is to
/// separate — its space above is what makes it read as a boundary.
pub const SECTION_SPACING_ABOVE: f32 = 18.0;

/// Left inset shared by the sidebar heading and the category buttons, so the
/// two cannot drift out of alignment.
pub const SIDEBAR_PADDING_X: f32 = 12.0;

/// Fixed width for a settings row's label column.
///
/// Without this, each control starts immediately after its own label and a
/// column of dropdowns forms a staircase — labels range from "Thai" to
/// "Southeast Asian". A fixed column gives every control on a page one left
/// edge.
pub const LABEL_COLUMN_WIDTH: f32 = 140.0;

/// Internal padding for combo boxes, so the selected value is not flush
/// against the control's own border.
pub const CONTROL_PADDING: egui::Vec2 = egui::vec2(8.0, 4.0);

/// Space below a section heading, before its first row.
pub const SECTION_SPACING_BELOW: f32 = 6.0;

// ─────────────────────────────────────────────────────────────────────────────
// Building blocks
// ─────────────────────────────────────────────────────────────────────────────

/// A section heading within a settings page.
///
/// Set in semibold at chrome body size rather than a larger size: a settings
/// pane is not a document, and building a size hierarchy inside one makes it
/// compete with the app's actual headings. Weight and surrounding space carry
/// the structure instead.
pub fn section_heading(ui: &mut Ui, text: impl Into<String>) {
    ui.add_space(SECTION_SPACING_ABOVE);
    // Genuinely bold, via the registered bold family. `.strong()` only shifts
    // colour in egui and left these headings reading at body weight.
    ui.label(
        RichText::new(text)
            .font(crate::fonts::chrome_bold_font(chrome::SECTION))
            .color(ui.visuals().text_color()),
    );
    ui.add_space(SECTION_SPACING_BELOW);
}

/// A settings row whose label sits in a fixed-width column, so every control
/// on the page shares one left edge.
pub fn label_column(ui: &mut Ui, label: impl Into<String>) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(LABEL_COLUMN_WIDTH, ui.spacing().interact_size.y),
        egui::Sense::hover(),
    );
    ui.painter().text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label.into(),
        egui::FontId::proportional(chrome::BODY),
        ui.visuals().text_color(),
    );
}

/// Apply settings-wide control metrics to this `Ui`.
///
/// Call once per settings page. Sets combo-box padding and slider width
/// centrally rather than per control, so a new control picks up the house
/// style by default instead of by being remembered.
pub fn apply_control_style(ui: &mut Ui) {
    ui.spacing_mut().button_padding = CONTROL_PADDING;
    ui.spacing_mut().slider_width = SLIDER_WIDTH;
}

/// Explanatory text under a heading or beside a control.
///
/// Wraps by default — the code-execution warning previously ran off the right
/// edge of the panel because it was a non-wrapping label.
pub fn description(ui: &mut Ui, text: impl Into<String>) {
    ui.label(
        RichText::new(text)
            .size(chrome::SMALL)
            .color(ui.visuals().weak_text_color()),
    );
}

/// A warning that must be read before acting — used for the code-execution
/// notice, which is a genuine security caveat rather than a hint.
///
/// Takes the theme's error colour rather than a hardcoded yellow: the previous
/// amber measured poorly against the warm page and is invisible to some
/// readers against it.
pub fn warning(ui: &mut Ui, text: impl Into<String>, color: Color32) {
    ui.label(RichText::new(text).size(chrome::SMALL).color(color));
}

/// Alternating row background for long *lists*.
///
/// Deliberately not for grids. Banding a two-column checkbox grid tints pairs
/// of unrelated settings, and reads as a highlight on whichever setting happens
/// to land on an odd row rather than as a guide along a row.
///
/// `index` is the row's position in its list. Odd rows take
/// `faint_bg_color` — around 1.07:1 against the page, which is a tint rather
/// than a block, and is exactly what that visual is for.
///
/// Banding matters most where a row's label and its value sit far apart, as in
/// the keyboard list: without it the eye has to track across empty space and
/// loses the line.
pub fn row<R>(ui: &mut Ui, index: usize, add_contents: impl FnOnce(&mut Ui) -> R) -> R {
    let striped = index % 2 == 1;
    let fill = if striped {
        ui.visuals().faint_bg_color
    } else {
        Color32::TRANSPARENT
    };

    egui::Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(
            ROW_PADDING_X as i8,
            ROW_PADDING_Y as i8,
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Rows must fill the available width or the stripe stops at the
                // content and the banding looks like a highlight on the label.
                ui.set_min_width(ui.available_width());
                add_contents(ui)
            })
            .inner
        })
        .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::typescale::chrome;

    #[test]
    fn control_width_fits_the_settings_pane() {
        // The settings content area is narrower than the window; a control
        // wider than this would clip or force horizontal scrolling.
        assert!(CONTROL_WIDTH <= 260.0);
        assert!(SLIDER_WIDTH < CONTROL_WIDTH);
    }

    #[test]
    fn section_spacing_separates_more_above_than_below() {
        // A heading belongs to the content beneath it, so it must sit closer to
        // that than to the section it follows.
        assert!(SECTION_SPACING_ABOVE > SECTION_SPACING_BELOW);
    }

    #[test]
    fn text_sizes_respect_the_chrome_minimum() {
        assert!(chrome::SMALL >= chrome::MINIMUM);
        assert!(chrome::BODY >= chrome::MINIMUM);
    }
}
