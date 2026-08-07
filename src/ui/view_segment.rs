//! View Mode Segmented Control for Ferrite
//!
//! This module implements a modern segmented control widget for switching
//! between view modes (Raw, Rendered, Split, LiveMarkdown) in the title bar.
//!
//! The control displays all four view modes as a compact pill-shaped button group,
//! making it immediately clear which modes are available and which is currently active.

use crate::app::modifier_symbol;
use crate::config::ViewMode;
use crate::fonts::FONT_INTER;
use crate::state::FileType;
use crate::theme::accent;
use crate::ui::icons::phosphor_font;
use crate::ui::phosphor_icons::{PAUSE, TIMER};
use eframe::egui::{
    self, Color32, FontFamily, FontId, Response, RichText, Sense, Vec2, WidgetInfo, WidgetType,
};
use rust_i18n::t;

/// Height of the segmented control.
const SEGMENT_HEIGHT: f32 = 24.0;

/// Width of each segment button when falling back to icon-only rendering
/// (labels too wide to fit the reserved title-bar space).
const SEGMENT_WIDTH: f32 = 26.0;

/// Horizontal padding added on each side of a measured label to form a
/// segment's clickable width.
const LABEL_H_PADDING: f32 = 14.0;

/// If the summed width of all label segments would exceed this, fall back to
/// icon-only rendering. Long-word locales (German, Finnish, ...) can blow the
/// title bar's reserved button budget otherwise.
const MAX_LABELED_TOTAL_WIDTH: f32 = 200.0;

/// Font used to draw the labelled segments.
fn label_font() -> FontId {
    FontId::new(11.0, FontFamily::Name(FONT_INTER.into()))
}

/// Localized label for a view mode, used by the segmented control.
fn localized_label(mode: ViewMode) -> String {
    match mode {
        ViewMode::Raw => t!("view_mode.label_raw").to_string(),
        ViewMode::Rendered => t!("view_mode.label_rendered").to_string(),
        ViewMode::Split => t!("view_mode.label_split").to_string(),
        ViewMode::LiveMarkdown => t!("view_mode.label_live").to_string(),
    }
}

/// Corner rounding for the pill shape.
const CORNER_ROUNDING: f32 = 10.0;

/// Inner padding for the selected indicator.
const INNER_PADDING: f32 = 2.0;

/// Actions that can be triggered from the view mode segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewSegmentAction {
    /// Switch to Raw mode
    SetRaw,
    /// Switch to Split mode
    SetSplit,
    /// Switch to Rendered mode
    SetRendered,
    /// Switch to LiveMarkdown mode
    SetLiveMarkdown,
}

/// Map a `ViewMode` to the action that switches to it.
fn action_for_mode(mode: ViewMode) -> ViewSegmentAction {
    match mode {
        ViewMode::Raw => ViewSegmentAction::SetRaw,
        ViewMode::Split => ViewSegmentAction::SetSplit,
        ViewMode::Rendered => ViewSegmentAction::SetRendered,
        ViewMode::LiveMarkdown => ViewSegmentAction::SetLiveMarkdown,
    }
}

/// View mode segmented control widget.
///
/// A four-button toggle control for switching between Raw, Rendered, Split, and
/// LiveMarkdown views.
/// File-type aware: disables Split for non-markdown structured files.
#[derive(Debug, Clone, Default)]
pub struct ViewModeSegment;

impl ViewModeSegment {
    /// Create a new view mode segment widget.
    pub fn new() -> Self {
        Self
    }

    /// Show the segmented control and return any triggered action.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `current_mode` - The currently active view mode
    /// * `file_type` - The current file type (affects which modes are available)
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// Optional action if a segment was clicked
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        current_mode: ViewMode,
        file_type: FileType,
        is_dark: bool,
        ferrite_accent: Color32,
    ) -> Option<ViewSegmentAction> {
        let mut action: Option<ViewSegmentAction> = None;

        // Determine which modes are available based on file type
        let split_available = file_type.is_markdown() || file_type.is_tabular();
        let rendered_available =
            file_type.is_markdown() || file_type.is_structured() || file_type.is_tabular();

        // Colors - refined for a polished pill appearance
        let bg_color = if is_dark {
            Color32::from_rgb(45, 45, 48) // Subtle dark background
        } else {
            Color32::from_rgb(228, 228, 231) // Light gray background
        };

        let selected_bg = if is_dark {
            ferrite_accent
        } else {
            accent::lerp_color(ferrite_accent, Color32::WHITE, 0.2)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(55, 55, 60)
        } else {
            Color32::from_rgb(240, 240, 243)
        };

        let text_color = if is_dark {
            Color32::from_rgb(200, 200, 200)
        } else {
            Color32::from_rgb(60, 60, 65)
        };

        // Derived from the fill it sits on, not hard-coded: white on the
        // default accent measures 2.2:1, and the accent is user-configurable.
        let selected_text = accent::on_accent(selected_bg);

        let disabled_text = if is_dark {
            Color32::from_rgb(80, 80, 85)
        } else {
            Color32::from_rgb(170, 170, 175)
        };

        // Border/shadow for depth
        let border_color = if is_dark {
            Color32::from_rgb(35, 35, 38)
        } else {
            Color32::from_rgb(200, 200, 205)
        };

        // Define segment data: (mode, icon, tooltip, action, enabled), driven by
        // `ViewMode::all()` so every mode always has a segment to select.
        let modes = ViewMode::all();
        let segments: Vec<(ViewMode, &'static str, String, ViewSegmentAction, bool)> = modes
            .iter()
            .map(|&mode| {
                let enabled = match mode {
                    ViewMode::Split => split_available,
                    ViewMode::Rendered => rendered_available,
                    ViewMode::Raw | ViewMode::LiveMarkdown => true,
                };
                (
                    mode,
                    mode.icon(),
                    format!("{} — {}", mode.label(), mode.description()),
                    action_for_mode(mode),
                    enabled,
                )
            })
            .collect();

        // Labels are localized words ("Raw", "Rendered", ...) measured at
        // runtime, because long-word locales (German, Finnish, ...) can blow
        // the title bar's reserved button budget. If the summed labelled
        // width would exceed `MAX_LABELED_TOTAL_WIDTH`, fall back to the
        // fixed-width Phosphor glyph rendering used before this change.
        let labels: Vec<String> = segments.iter().map(|(mode, ..)| localized_label(*mode)).collect();
        let label_widths: Vec<f32> = labels
            .iter()
            .map(|label| {
                let galley =
                    ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), label_font(), Color32::WHITE));
                galley.size().x + LABEL_H_PADDING
            })
            .collect();
        let labeled_total_width: f32 = label_widths.iter().sum();
        let use_labels = labeled_total_width <= MAX_LABELED_TOTAL_WIDTH;

        let segment_widths: Vec<f32> = if use_labels {
            label_widths
        } else {
            vec![SEGMENT_WIDTH; segments.len()]
        };
        let total_width: f32 = segment_widths.iter().sum();
        let size = Vec2::new(total_width, SEGMENT_HEIGHT);

        // Allocate space for the entire control
        let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());

        // Draw outer border/shadow for depth
        ui.painter()
            .rect_filled(rect.expand(1.0), CORNER_ROUNDING + 1.0, border_color);

        // Draw pill background
        ui.painter().rect_filled(rect, CORNER_ROUNDING, bg_color);

        // First pass: draw selected indicator (so it appears behind icons).
        // If the current mode is somehow missing from `segments`, no pill is
        // drawn, but the segments themselves are still rendered below.
        let mut x_offset = rect.min.x;
        for ((mode, _, _, _, _), &width) in segments.iter().zip(segment_widths.iter()) {
            if current_mode == *mode {
                let segment_rect = egui::Rect::from_min_size(
                    egui::pos2(x_offset, rect.min.y),
                    Vec2::new(width, SEGMENT_HEIGHT),
                );

                // Inset the selected indicator slightly for a "floating pill" effect
                let indicator_rect = segment_rect.shrink(INNER_PADDING);

                // Draw selected pill with slight shadow effect in light mode
                if !is_dark {
                    // Subtle shadow
                    ui.painter().rect_filled(
                        indicator_rect.translate(Vec2::new(0.0, 1.0)),
                        CORNER_ROUNDING - INNER_PADDING,
                        Color32::from_rgba_unmultiplied(0, 0, 0, 15),
                    );
                }

                ui.painter().rect_filled(
                    indicator_rect,
                    CORNER_ROUNDING - INNER_PADDING,
                    selected_bg,
                );
                break;
            }
            x_offset += width;
        }

        // Second pass: draw labels/icons and handle interactions
        x_offset = rect.min.x;
        for (((mode, icon, tooltip, segment_action, enabled), &width), label) in
            segments.iter().zip(segment_widths.iter()).zip(labels.iter())
        {
            let segment_rect = egui::Rect::from_min_size(
                egui::pos2(x_offset, rect.min.y),
                Vec2::new(width, SEGMENT_HEIGHT),
            );

            let is_selected = current_mode == *mode;

            // Create clickable area
            let segment_response = ui.allocate_rect(segment_rect, Sense::click());
            segment_response.widget_info(|| {
                WidgetInfo::labeled(WidgetType::RadioButton, *enabled, label.as_str())
            });

            // Draw hover effect (only for non-selected, enabled segments)
            if !is_selected && segment_response.hovered() && *enabled {
                let hover_rect = segment_rect.shrink(INNER_PADDING);
                ui.painter()
                    .rect_filled(hover_rect, CORNER_ROUNDING - INNER_PADDING, hover_bg);
            }

            // Determine text/icon color
            let content_color = if !*enabled {
                disabled_text
            } else if is_selected {
                selected_text
            } else {
                text_color
            };

            if use_labels {
                ui.painter().text(
                    segment_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    label_font(),
                    content_color,
                );
            } else {
                // Draw icon glyph (Phosphor font)
                ui.painter().text(
                    segment_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    phosphor_font(11.0),
                    content_color,
                );
            }

            // Handle click
            if segment_response.clicked() && *enabled && !is_selected {
                action = Some(*segment_action);
            }

            // Tooltip with keyboard shortcut
            let tooltip_text = if *enabled {
                format!("{} ({}+E to cycle)", tooltip, modifier_symbol())
            } else {
                format!("{} (not available for this file type)", tooltip)
            };
            // A focused segment must be locatable without a pointer.
            crate::ui::a11y::focus_ring(ui, &segment_response, CORNER_ROUNDING as u8);
            segment_response.on_hover_text(tooltip_text);

            x_offset += width;
        }

        action
    }

    /// Show a compact 2-mode segment for structured files (Raw/Rendered only).
    ///
    /// This variant is used when Split mode is not available.
    pub fn show_two_mode(
        &self,
        ui: &mut egui::Ui,
        current_mode: ViewMode,
        is_dark: bool,
        ferrite_accent: Color32,
    ) -> Option<ViewSegmentAction> {
        let mut action: Option<ViewSegmentAction> = None;

        // Colors
        let bg_color = if is_dark {
            Color32::from_rgb(45, 45, 48)
        } else {
            Color32::from_rgb(228, 228, 231)
        };

        let selected_bg = if is_dark {
            ferrite_accent
        } else {
            accent::lerp_color(ferrite_accent, Color32::WHITE, 0.2)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(55, 55, 60)
        } else {
            Color32::from_rgb(240, 240, 243)
        };

        let text_color = if is_dark {
            Color32::from_rgb(200, 200, 200)
        } else {
            Color32::from_rgb(60, 60, 65)
        };

        // Derived from the fill it sits on, not hard-coded: white on the
        // default accent measures 2.2:1, and the accent is user-configurable.
        let selected_text = accent::on_accent(selected_bg);

        let border_color = if is_dark {
            Color32::from_rgb(35, 35, 38)
        } else {
            Color32::from_rgb(200, 200, 205)
        };

        // Two segments only
        let total_width = SEGMENT_WIDTH * 2.0;
        let size = Vec2::new(total_width, SEGMENT_HEIGHT);

        let (rect, _response) = ui.allocate_exact_size(size, Sense::hover());

        // Draw border and background
        ui.painter()
            .rect_filled(rect.expand(1.0), CORNER_ROUNDING + 1.0, border_color);
        ui.painter().rect_filled(rect, CORNER_ROUNDING, bg_color);

        let segments = [
            (ViewMode::Raw, "R", "Raw Editor", ViewSegmentAction::SetRaw),
            (
                ViewMode::Rendered,
                "V",
                "Rendered View",
                ViewSegmentAction::SetRendered,
            ),
        ];

        // Draw selected indicator
        let mut x_offset = rect.min.x;
        for (mode, _, _, _) in segments.iter() {
            if current_mode == *mode || (current_mode == ViewMode::Split && *mode == ViewMode::Raw)
            {
                let segment_rect = egui::Rect::from_min_size(
                    egui::pos2(x_offset, rect.min.y),
                    Vec2::new(SEGMENT_WIDTH, SEGMENT_HEIGHT),
                );
                let indicator_rect = segment_rect.shrink(INNER_PADDING);

                if !is_dark {
                    ui.painter().rect_filled(
                        indicator_rect.translate(Vec2::new(0.0, 1.0)),
                        CORNER_ROUNDING - INNER_PADDING,
                        Color32::from_rgba_unmultiplied(0, 0, 0, 15),
                    );
                }

                ui.painter().rect_filled(
                    indicator_rect,
                    CORNER_ROUNDING - INNER_PADDING,
                    selected_bg,
                );
                break;
            }
            x_offset += SEGMENT_WIDTH;
        }

        // Draw icons and handle interactions
        x_offset = rect.min.x;
        for (mode, icon, tooltip, segment_action) in segments.iter() {
            let segment_rect = egui::Rect::from_min_size(
                egui::pos2(x_offset, rect.min.y),
                Vec2::new(SEGMENT_WIDTH, SEGMENT_HEIGHT),
            );

            let is_selected = current_mode == *mode
                || (current_mode == ViewMode::Split && *mode == ViewMode::Raw);
            let segment_response = ui.allocate_rect(segment_rect, Sense::click());

            if !is_selected && segment_response.hovered() {
                let hover_rect = segment_rect.shrink(INNER_PADDING);
                ui.painter()
                    .rect_filled(hover_rect, CORNER_ROUNDING - INNER_PADDING, hover_bg);
            }

            let icon_color = if is_selected {
                selected_text
            } else {
                text_color
            };

            ui.painter().text(
                segment_rect.center(),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(11.0),
                icon_color,
            );

            if segment_response.clicked() && !is_selected {
                action = Some(*segment_action);
            }

            let tooltip_text = format!("{} ({}+E to toggle)", tooltip, modifier_symbol());
            // A focused segment must be locatable without a pointer.
            crate::ui::a11y::focus_ring(ui, &segment_response, CORNER_ROUNDING as u8);
            segment_response.on_hover_text(tooltip_text);

            x_offset += SEGMENT_WIDTH;
        }

        action
    }
}

/// Title bar button for quick toggles (Settings, Zen Mode, etc.)
///
/// A compact button styled for the title bar area.
pub struct TitleBarButton;

impl TitleBarButton {
    /// Show a title bar toggle button.
    ///
    /// # Arguments
    ///
    /// * `ui` - The egui UI context
    /// * `icon` - The icon/emoji to display
    /// * `tooltip` - Hover tooltip text
    /// * `is_active` - Whether the button is in active/on state
    /// * `is_dark` - Whether the current theme is dark mode
    ///
    /// # Returns
    ///
    /// The button response for click detection
    pub fn show(
        ui: &mut egui::Ui,
        icon: &str,
        tooltip: &str,
        is_active: bool,
        is_dark: bool,
    ) -> Response {
        let size = Vec2::new(28.0, 24.0); // Slightly taller for better alignment

        let text_color = if is_dark {
            Color32::from_rgb(220, 220, 220)
        } else {
            Color32::from_rgb(40, 40, 40)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(210, 210, 210)
        };

        let active_bg = if is_dark {
            Color32::from_rgb(60, 90, 60) // Green-ish for active
        } else {
            Color32::from_rgb(200, 230, 200)
        };

        let btn = ui.add(
            egui::Button::new(RichText::new(" ").size(14.0)) // Match icon size
                .frame(false)
                .min_size(size),
        );

        // Draw background on hover or if active
        if is_active {
            ui.painter()
                .rect_filled(btn.rect, egui::CornerRadius::same(3), active_bg);
        } else if btn.hovered() {
            ui.painter()
                .rect_filled(btn.rect, egui::CornerRadius::same(3), hover_bg);
        }

        // Draw icon centered
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            phosphor_font(14.0),
            text_color,
        );

        // The button's own text is a single space (it exists only to reserve a
        // hit area), so without this the control announces as `" "`.
        crate::ui::a11y::name_toggle(&btn, tooltip, true, is_active);
        crate::ui::a11y::focus_ring(ui, &btn, 3);

        btn.on_hover_text(tooltip)
    }

    /// Show an auto-save indicator button with special styling.
    ///
    /// Green tint when enabled, muted when disabled.
    pub fn show_auto_save(ui: &mut egui::Ui, enabled: bool, is_dark: bool) -> Response {
        let size = Vec2::new(28.0, 24.0); // Match other title bar buttons

        let icon = if enabled { TIMER } else { PAUSE };
        let tooltip = if enabled {
            "Auto-Save: ON (click to disable)"
        } else {
            "Auto-Save: OFF (click to enable)"
        };

        // Green tint for enabled, muted for disabled
        let text_color = if enabled {
            if is_dark {
                Color32::from_rgb(100, 200, 100) // Green
            } else {
                Color32::from_rgb(40, 140, 40)
            }
        } else if is_dark {
            Color32::from_rgb(120, 120, 120) // Muted
        } else {
            Color32::from_rgb(140, 140, 140)
        };

        let hover_bg = if is_dark {
            Color32::from_rgb(60, 60, 60)
        } else {
            Color32::from_rgb(210, 210, 210)
        };

        let btn = ui.add(
            egui::Button::new(RichText::new(" ").size(14.0)) // Match icon size
                .frame(false)
                .min_size(size),
        );

        if btn.hovered() {
            ui.painter()
                .rect_filled(btn.rect, egui::CornerRadius::same(3), hover_bg);
        }

        // Draw icon centered with appropriate color
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            phosphor_font(14.0),
            text_color,
        );

        crate::ui::a11y::name_widget(&btn, tooltip, true);
        crate::ui::a11y::focus_ring(ui, &btn, 3);

        btn.on_hover_text(tooltip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_mode_segment_new() {
        let segment = ViewModeSegment::new();
        // Just verify it can be created
        let _ = segment;
    }

    #[test]
    fn test_view_segment_action_equality() {
        assert_eq!(ViewSegmentAction::SetRaw, ViewSegmentAction::SetRaw);
        assert_ne!(ViewSegmentAction::SetRaw, ViewSegmentAction::SetSplit);
    }

    #[test]
    fn test_view_mode_segment_default() {
        let segment = ViewModeSegment::default();
        let _ = segment;
    }
}
