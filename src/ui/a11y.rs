//! Accessibility helpers for custom-painted controls.
//!
//! This app draws most of its chrome by hand — `Button::new(RichText::new(" "))`
//! to reserve a hit area, then `painter()` calls for the glyph. That gives full
//! control over the look, and costs two things egui would otherwise provide:
//!
//! 1. **A name.** egui derives a widget's accessible name from its text, so a
//!    button whose text is a single space reports `" "` to AccessKit. The label
//!    exists, but only as a tooltip, which assistive technology never sees.
//! 2. **A focus indicator.** A bare `allocate_rect` / `interact` can take
//!    keyboard focus and render identically focused or not, leaving a keyboard
//!    user with no idea where they are.
//!
//! Both helpers here take the [`Response`] a control already has, so adding
//! them to an existing widget is one line and changes no layout.

use eframe::egui::{self, Response, Ui};

/// Give a hand-painted control an accessible name.
///
/// Use the same string as the tooltip: a control should not describe itself one
/// way to sighted users and another to a screen reader.
///
/// `enabled` is reported to AccessKit so a disabled control announces as such
/// rather than appearing actionable.
pub fn name_widget(response: &Response, label: impl Into<String>, enabled: bool) {
    let label = label.into();
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &label)
    });
}

/// Name a control that toggles, so its on/off state is announced.
pub fn name_toggle(response: &Response, label: impl Into<String>, enabled: bool, on: bool) {
    let label = label.into();
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, &label)
    });
}

/// Draw a focus ring around `rect` when `response` holds keyboard focus.
///
/// Uses the theme's selection stroke so the ring follows the user's accent, and
/// insets by half the stroke width so it sits inside the control's own bounds
/// rather than bleeding into whatever is drawn beside it.
///
/// Call this *after* painting the control's background so the ring is not
/// covered by it.
pub fn focus_ring(ui: &Ui, response: &Response, corner_radius: u8) {
    if !response.has_focus() {
        return;
    }
    let stroke = ui.visuals().selection.stroke;
    let rect = response.rect.shrink(stroke.width * 0.5);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(corner_radius),
        stroke,
        egui::StrokeKind::Inside,
    );
}

/// Minimum interactive target size, per WCAG 2.2 §2.5.8.
///
/// Applies to anything clickable. Controls in dense surfaces (the file tree's
/// rows, for instance) may legitimately sit below this where raising them would
/// change information density materially — but that should be a decision, not
/// an accident.
pub const MIN_TARGET_SIZE: f32 = 24.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_target_size_matches_wcag() {
        assert_eq!(MIN_TARGET_SIZE, 24.0);
    }
}
