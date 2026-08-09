//! Merged file/format toolbar - single top-of-window chrome bar.
//!
//! Replaces the historical split between the ribbon (file/tool actions) and
//! the format toolbar (markdown formatting buttons): two icon strips ~60px
//! apart that existed only because the formatting buttons were moved out of
//! the ribbon at some point, not because of any design intent (see
//! `ribbon.rs`, which still holds the shared `RibbonAction` enum). This
//! module now renders both halves in one 32px bar:
//!
//! `[New] [Open] [Open Folder] [Save▾] [Find] | [B] [I] [</>] [Link] |
//! [H▾] [Bullet] [Numbered] [Quote] [TOC] | [Mermaid▾] ... [Export▾] [Terminal]`
//!
//! The formatting half (bold/italic/.../Mermaid) only renders for markdown
//! files; the file/tool half always renders. There is no collapsed state —
//! the bar is either shown (one view, one height) or not rendered at all
//! (Zen Mode, handled by the caller).

use crate::app::modifier_symbol;
use crate::markdown::formatting::{FormattingState, MarkdownFormatCommand};
use crate::markdown::mermaid::{mermaid_kind_menu_label, MermaidTemplateKind};
use crate::state::FileType;
use crate::theme::{accent, ThemeColors};
use crate::ui::phosphor_icons::{
    phosphor_font, phosphor_rich_text, CARET_LEFT, CODE_BLOCK,
};
use crate::ui::skrivr_icons;
use crate::ui::RibbonAction;
use eframe::egui::{self, Color32, FontId, RichText, Ui, Vec2};
use egui_phosphor::regular::{
    CHECK, CLIPBOARD, EXPORT, FILE_MAGNIFYING_GLASS, FILE_PDF, FILE_PLUS, FILE_TEXT, FLOPPY_DISK,
    FOLDERS, FOLDER_SIMPLE_MINUS, GLOBE, LIGHTNING, MAGNIFYING_GLASS, PRINTER, SPARKLE,
    TERMINAL_WINDOW,
};
use rust_i18n::t;

/// Height of the merged toolbar.
pub const TOOLBAR_HEIGHT: f32 = 32.0;

/// Which icon font a format-toolbar glyph is drawn from.
///
/// The bundled Skrivr glyphs are fitted to a uniform em square and render
/// optically larger than Phosphor's at an equal point size, so the two fonts
/// need different point sizes to look consistent when they sit in the same
/// row (see `format_button`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum IconGlyphFont {
    Phosphor,
    Skrivr,
}

impl IconGlyphFont {
    fn size(self) -> f32 {
        match self {
            // Raised from 11/13: the glyphs read as undersized in a 32px bar,
            // and the two sets are kept one step apart because the Skrivr set
            // is fitted to a uniform em square and renders optically smaller
            // than Phosphor at the same nominal size.
            IconGlyphFont::Phosphor => 15.0,
            IconGlyphFont::Skrivr => 16.0,
        }
    }

    fn font_id(self) -> FontId {
        match self {
            IconGlyphFont::Phosphor => phosphor_font(self.size()),
            IconGlyphFont::Skrivr => skrivr_icons::skrivr_font(self.size()),
        }
    }
}

/// Format toolbar component for the top of the raw editor.
pub struct FormatToolbar;

impl FormatToolbar {
    /// Render the merged file/format bar.
    ///
    /// `formatting_state` is `None` when there is nothing sensible to derive
    /// it from (no editor, or a non-markdown file) — the formatting buttons
    /// then just render disabled/inactive rather than being hidden, except
    /// for the markdown-only group, which is skipped entirely when
    /// `file_type` is not markdown.
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut Ui,
        colors: &ThemeColors,
        has_editor: bool,
        is_workspace_mode: bool,
        file_type: FileType,
        pipeline_enabled: bool,
        formatting_state: Option<&FormattingState>,
    ) -> FormatToolbarOutput {
        let mut action: Option<RibbonAction> = None;

        let bar_bg = colors.base.background_secondary;
        let separator_color = colors.base.border;

        let rect = ui.available_rect_before_wrap();
        let _response = ui.allocate_rect(rect, egui::Sense::hover());

        // Background
        ui.painter().rect_filled(rect, 0.0, bar_bg);

        // Bottom border
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            egui::Stroke::new(1.0, separator_color),
        );

        let mut button_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(Vec2::new(4.0, 2.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        // Icons need room to read as separate controls; at 2px they
            // crowded into a single band.
            button_ui.spacing_mut().item_spacing.x = 4.0;

        // ── File / workspace / tool actions ─────────────────────────────
        if file_icon_button(
            &mut button_ui,
            FILE_PLUS,
            &format!("New ({}+N)", modifier_symbol()),
            true,
            colors,
        )
        .clicked()
        {
            action = Some(RibbonAction::New);
        }

        if file_icon_button(
            &mut button_ui,
            FILE_TEXT,
            &format!("Open File ({}+O)", modifier_symbol()),
            true,
            colors,
        )
        .clicked()
        {
            action = Some(RibbonAction::Open);
        }

        if is_workspace_mode {
            if file_icon_button(&mut button_ui, FOLDER_SIMPLE_MINUS, "Close Workspace", true, colors)
                .clicked()
            {
                action = Some(RibbonAction::CloseWorkspace);
            }
        } else if file_icon_button(
            &mut button_ui,
            FOLDERS,
            &format!("Open Folder ({}+Shift+O)", modifier_symbol()),
            true,
            colors,
        )
        .clicked()
        {
            action = Some(RibbonAction::OpenWorkspace);
        }

        if is_workspace_mode {
            if file_icon_button(
                &mut button_ui,
                FILE_MAGNIFYING_GLASS,
                &format!("Search in Files ({}+Shift+F)", modifier_symbol()),
                true,
                colors,
            )
            .clicked()
            {
                action = Some(RibbonAction::SearchInFiles);
            }

            if file_icon_button(
                &mut button_ui,
                LIGHTNING,
                &format!("Quick File Switcher ({}+P)", modifier_symbol()),
                true,
                colors,
            )
            .clicked()
            {
                action = Some(RibbonAction::QuickFileSwitcher);
            }
        }

        // Save dropdown
        egui::ComboBox::from_id_salt("toolbar_save_dropdown")
            .selected_text(phosphor_rich_text(FLOPPY_DISK, 14.0))
            .width(40.0)
            .show_ui(&mut button_ui, |ui| {
                if ui
                    .selectable_label(false, t!("menu.file.save"))
                    .on_hover_text(format!("Save ({}+S)", modifier_symbol()))
                    .clicked()
                {
                    action = Some(RibbonAction::Save);
                }
                if ui
                    .selectable_label(false, format!("{}...", t!("menu.file.save_as")))
                    .on_hover_text(format!("Save As ({}+Shift+S)", modifier_symbol()))
                    .clicked()
                {
                    action = Some(RibbonAction::SaveAs);
                }
            });

        // Structured data operations (JSON/YAML/TOML only)
        if file_type.is_structured() {
            button_ui.add_space(7.0);
            toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
            button_ui.add_space(7.0);

            if file_icon_button(
                &mut button_ui,
                SPARKLE,
                &t!("ribbon.format_document").to_string(),
                has_editor,
                colors,
            )
            .clicked()
            {
                action = Some(RibbonAction::FormatDocument);
            }

            if file_icon_button(
                &mut button_ui,
                CHECK,
                &t!("ribbon.validate_syntax").to_string(),
                has_editor,
                colors,
            )
            .clicked()
            {
                action = Some(RibbonAction::ValidateSyntax);
            }

            if matches!(file_type, FileType::Json | FileType::Yaml)
                && file_icon_button(
                    &mut button_ui,
                    LIGHTNING,
                    &format!("{} ({}+Shift+L)", t!("ribbon.pipeline"), modifier_symbol()),
                    has_editor && pipeline_enabled,
                    colors,
                )
                .clicked()
            {
                action = Some(RibbonAction::TogglePipeline);
            }
        }

        button_ui.add_space(7.0);
        toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
        button_ui.add_space(7.0);

        if file_icon_button(
            &mut button_ui,
            MAGNIFYING_GLASS,
            &format!("Find/Replace ({}+F)", modifier_symbol()),
            true,
            colors,
        )
        .clicked()
        {
            action = Some(RibbonAction::FindReplace);
        }

        // ── Markdown formatting half (markdown files only) ──────────────
        if file_type.is_markdown() {
            button_ui.add_space(7.0);
            toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
            button_ui.add_space(7.0);

            let is_bold = formatting_state.map(|s| s.is_bold).unwrap_or(false);
            let is_italic = formatting_state.map(|s| s.is_italic).unwrap_or(false);
            let is_code = formatting_state.map(|s| s.is_inline_code).unwrap_or(false);
            let is_link = formatting_state.map(|s| s.is_link).unwrap_or(false);

            if format_button(
                &mut button_ui,
                skrivr_icons::BOLD,
                &MarkdownFormatCommand::Bold.tooltip(),
                has_editor,
                is_bold,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Bold));
            }

            if format_button(
                &mut button_ui,
                skrivr_icons::ITALIC,
                &MarkdownFormatCommand::Italic.tooltip(),
                has_editor,
                is_italic,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Italic));
            }

            if format_button(
                &mut button_ui,
                skrivr_icons::CODE_BLOCK,
                &MarkdownFormatCommand::InlineCode.tooltip(),
                has_editor,
                is_code,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::InlineCode));
            }

            if format_button(
                &mut button_ui,
                skrivr_icons::LINK,
                &MarkdownFormatCommand::Link.tooltip(),
                has_editor,
                is_link,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Link));
            }

            button_ui.add_space(7.0);
            toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
            button_ui.add_space(7.0);

            // Heading dropdown
            let current_heading = formatting_state.and_then(|s| s.heading_level);
            let heading_label = current_heading
                .map(|h| format!("H{}", h as u8))
                .unwrap_or_else(|| "H".to_string());

            egui::ComboBox::from_id_salt("format_bar_heading_dropdown")
                .selected_text(RichText::new(heading_label).size(11.0))
                .width(36.0)
                .show_ui(&mut button_ui, |ui| {
                    for level in 1..=6u8 {
                        let is_selected =
                            current_heading.map(|h| h as u8 == level).unwrap_or(false);
                        let label = format!("H{}", level);
                        if ui
                            .selectable_label(is_selected, &label)
                            .on_hover_text(format!("{}+{}", modifier_symbol(), level))
                            .clicked()
                        {
                            action =
                                Some(RibbonAction::Format(MarkdownFormatCommand::Heading(level)));
                        }
                    }
                });

            button_ui.add_space(7.0);
            toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
            button_ui.add_space(7.0);

            // List buttons
            let is_bullet = formatting_state.map(|s| s.is_bullet_list).unwrap_or(false);
            let is_numbered = formatting_state
                .map(|s| s.is_numbered_list)
                .unwrap_or(false);

            if format_button(
                &mut button_ui,
                skrivr_icons::LIST_UNORDERED,
                &MarkdownFormatCommand::BulletList.tooltip(),
                has_editor,
                is_bullet,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::BulletList));
            }

            if format_button(
                &mut button_ui,
                skrivr_icons::LIST_NUMBERED,
                &MarkdownFormatCommand::NumberedList.tooltip(),
                has_editor,
                is_numbered,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::NumberedList));
            }

            // Blockquote
            let is_quote = formatting_state.map(|s| s.is_blockquote).unwrap_or(false);
            if format_button(
                &mut button_ui,
                skrivr_icons::QUOTE,
                &MarkdownFormatCommand::Blockquote.tooltip(),
                has_editor,
                is_quote,
                colors,
                IconGlyphFont::Skrivr,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::Blockquote));
            }

            // Code block
            let is_code_block = formatting_state.map(|s| s.is_code_block).unwrap_or(false);
            if format_button(
                &mut button_ui,
                CODE_BLOCK,
                &MarkdownFormatCommand::CodeBlock.tooltip(),
                has_editor,
                is_code_block,
                colors,
                IconGlyphFont::Phosphor,
            )
            .clicked()
            {
                action = Some(RibbonAction::Format(MarkdownFormatCommand::CodeBlock));
            }

            button_ui.add_space(7.0);
            toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
            button_ui.add_space(7.0);

            // Table of Contents
            if toolbar_icon_button(
                &mut button_ui,
                skrivr_icons::OUTLINE,
                &format!(
                    "Insert/Update Table of Contents ({}+Shift+U)",
                    modifier_symbol()
                ),
                has_editor,
                colors,
            )
            .clicked()
            {
                action = Some(RibbonAction::InsertToc);
            }

            button_ui.add_space(7.0);
            toolbar_separator(&mut button_ui, separator_color, TOOLBAR_HEIGHT - 12.0);
            button_ui.add_space(7.0);

            // Mermaid diagram templates
            egui::ComboBox::from_id_salt("format_bar_mermaid_dropdown")
                .selected_text(
                    RichText::new(t!("format_toolbar.mermaid_menu").to_string()).size(11.0),
                )
                .width(108.0)
                .show_ui(&mut button_ui, |ui| {
                    ui.set_min_width(160.0);
                    for &kind in MermaidTemplateKind::ALL {
                        let label = mermaid_kind_menu_label(kind);
                        if ui
                            .selectable_label(false, &label)
                            .on_hover_text(t!("format_toolbar.mermaid_entry_tooltip").to_string())
                            .clicked()
                        {
                            action = Some(RibbonAction::Format(
                                MarkdownFormatCommand::InsertMermaid(kind),
                            ));
                        }
                    }
                });
        }

        // ── Right-aligned: Export + Terminal ────────────────────────────
        button_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if file_icon_button(
                ui,
                TERMINAL_WINDOW,
                &format!("Toggle Terminal ({}+`)", modifier_symbol()),
                true,
                colors,
            )
            .clicked()
            {
                action = Some(RibbonAction::ToggleTerminal);
            }

            if file_type.is_markdown() {
                ui.add_space(4.0);
                toolbar_separator(ui, separator_color, TOOLBAR_HEIGHT - 12.0);
                ui.add_space(4.0);

                egui::ComboBox::from_id_salt("toolbar_export_dropdown")
                    .selected_text(phosphor_rich_text(EXPORT, 14.0))
                    .width(40.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                false,
                                format!("{} {}", GLOBE, t!("menu.file.export_html")),
                            )
                            .on_hover_text(format!(
                                "Export as HTML ({}+Shift+E)",
                                modifier_symbol()
                            ))
                            .clicked()
                        {
                            action = Some(RibbonAction::ExportHtml);
                        }
                        if ui
                            .selectable_label(
                                false,
                                format!("{} {}", CLIPBOARD, t!("menu.file.export_clipboard")),
                            )
                            .on_hover_text(t!("ribbon.copy_html_tooltip").to_string())
                            .clicked()
                        {
                            action = Some(RibbonAction::CopyAsHtml);
                        }
                        ui.separator();
                        if ui
                            .selectable_label(
                                false,
                                format!("{} {}", FILE_PDF, t!("ribbon.export_pdf")),
                            )
                            .on_hover_text(format!(
                                "Export as PDF ({}+Shift+P)",
                                modifier_symbol()
                            ))
                            .clicked()
                        {
                            action = Some(RibbonAction::ExportPdf);
                        }
                        if ui
                            .selectable_label(
                                false,
                                format!("{} {}", PRINTER, t!("ribbon.print_preview")),
                            )
                            .on_hover_text(format!(
                                "Print preview (+{}+Alt+P)",
                                modifier_symbol()
                            ))
                            .clicked()
                        {
                            action = Some(RibbonAction::PrintPreview);
                        }
                    });
            }
        });

        FormatToolbarOutput { action }
    }
}

/// Render a plain (non-toggling) file/tool icon button for the merged bar.
///
/// Mirrors the old ribbon's `icon_button`, but themed via `&ThemeColors`
/// instead of a raw `is_dark: bool`.
fn file_icon_button(ui: &mut Ui, icon: &str, tooltip: &str, enabled: bool, colors: &ThemeColors) -> egui::Response {
    let text_color = if enabled {
        colors.text.primary
    } else {
        colors.text.disabled
    };
    let hover_bg = colors.base.hover;

    let btn = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(" ").size(16.0))
            .frame(false)
            .min_size(Vec2::new(28.0, 24.0)),
    );

    if btn.hovered() && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::CornerRadius::same(3), hover_bg);
    }

    ui.painter().text(
        btn.rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        phosphor_font(14.0),
        text_color,
    );

    crate::ui::a11y::name_widget(&btn, tooltip, enabled);
    crate::ui::a11y::focus_ring(ui, &btn, 3);

    btn.on_hover_text(tooltip)
}

/// Output from the merged toolbar.
pub struct FormatToolbarOutput {
    /// Action triggered by a button click.
    pub action: Option<RibbonAction>,
}

/// Render a format button with active state highlighting.
fn format_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    active: bool,
    colors: &ThemeColors,
    icon_font: IconGlyphFont,
) -> egui::Response {
    let text_color = if enabled {
        colors.text.primary
    } else {
        colors.text.disabled
    };

    let active_bg = colors.base.selected;
    let active_text_color = accent::on_accent(colors.base.selected);
    let hover_bg = colors.base.hover;

    let font = icon_font.font_id();
    let text = RichText::new(icon).font(font.clone()).color(text_color);

    let btn = ui.add_enabled(
        enabled,
        egui::Button::new(text).frame(false).min_size(Vec2::new(
            crate::ui::a11y::MIN_TARGET_SIZE,
            crate::ui::a11y::MIN_TARGET_SIZE,
        )),
    );

    // `.frame(false)` disables egui's own press feedback, so without an
    // explicit pressed state a click produces no acknowledgement at all until
    // the document changes. Pressed reads darker than hover so the two are
    // distinguishable while the pointer is still down.
    let pressed = enabled && btn.is_pointer_button_down_on();

    if (active || pressed || btn.hovered()) && enabled {
        let fill = if pressed {
            crate::theme::accent::lerp_color(active_bg, Color32::BLACK, 0.12)
        } else if active {
            active_bg
        } else {
            hover_bg
        };
        let glyph = if active || pressed {
            active_text_color
        } else {
            text_color
        };
        ui.painter()
            .rect_filled(btn.rect, egui::CornerRadius::same(3), fill);
        ui.painter().text(
            btn.rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            font,
            glyph,
        );
    }

    crate::ui::a11y::name_toggle(&btn, tooltip, enabled, active);
    crate::ui::a11y::focus_ring(ui, &btn, 3);

    btn.on_hover_text(tooltip)
}

/// Small icon button for the toolbar.
fn toolbar_icon_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    colors: &ThemeColors,
) -> egui::Response {
    let text_color = if enabled {
        colors.text.primary
    } else {
        colors.text.disabled
    };

    let hover_bg = colors.base.hover;

    let btn = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(" ").size(14.0))
            .frame(false)
            .min_size(Vec2::new(
                crate::ui::a11y::MIN_TARGET_SIZE,
                crate::ui::a11y::MIN_TARGET_SIZE,
            )),
    );

    if btn.hovered() && enabled {
        ui.painter()
            .rect_filled(btn.rect, egui::CornerRadius::same(3), hover_bg);
    }

    ui.painter().text(
        btn.rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        skrivr_icons::skrivr_font(13.0),
        text_color,
    );

    crate::ui::a11y::name_widget(&btn, tooltip, enabled);
    crate::ui::a11y::focus_ring(ui, &btn, 3);

    btn.on_hover_text(tooltip)
}

/// Draw a vertical separator line in the toolbar.
fn toolbar_separator(ui: &mut Ui, color: Color32, height: f32) {
    let (rect, _response) = ui.allocate_exact_size(Vec2::new(1.0, height), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.center_top(), rect.center_bottom()],
        egui::Stroke::new(1.0, color),
    );
}

/// Render the side panel toggle strip (shown when the outline panel is closed).
///
/// Returns true if the user clicked to open the side panel.
///
/// When `blocks_clicks` is true (window resize cursor active), the strip ignores
/// clicks so east-edge resize is not mistaken for "open side panel".
pub fn side_panel_toggle_strip(ui: &mut egui::Ui, colors: &ThemeColors, blocks_clicks: bool) -> bool {
    let mut clicked = false;

    let strip_width = 20.0;

    let bg = colors.base.background_secondary;
    let separator_color = colors.base.border;
    let chevron_color = colors.text.muted;

    egui::Panel::right("side_panel_toggle_strip")
        .resizable(false)
        .exact_size(strip_width)
        .frame(
            egui::Frame::NONE
                .fill(bg)
                .stroke(egui::Stroke::NONE)
                .inner_margin(egui::Margin::ZERO),
        )
        .show_inside(ui, |ui| {
            // Left border
            let panel_rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [panel_rect.left_top(), panel_rect.left_bottom()],
                egui::Stroke::new(1.0, separator_color),
            );

            // Clickable area for the whole strip (disabled while resize cursor is active)
            let sense = if blocks_clicks {
                egui::Sense::hover()
            } else {
                egui::Sense::click()
            };
            let response = ui.allocate_rect(panel_rect, sense);

            // Hover effect
            if response.hovered() {
                ui.painter().rect_filled(panel_rect, 0.0, colors.base.hover);
            }

            // Chevron pointing left (to indicate "open panel")
            ui.painter().text(
                panel_rect.center(),
                egui::Align2::CENTER_CENTER,
                CARET_LEFT,
                phosphor_font(12.0),
                chevron_color,
            );

            if response.clicked() && !blocks_clicks {
                clicked = true;
            }

            response.on_hover_text(t!("format_toolbar.show_side_panel").to_string());
        });

    clicked
}
