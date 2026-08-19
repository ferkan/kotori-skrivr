//! Merged file/format toolbar - single top-of-window chrome bar.
//!
//! Replaces the historical split between the ribbon (file/tool actions) and
//! the format toolbar (markdown formatting buttons): two icon strips ~60px
//! apart that existed only because the formatting buttons were moved out of
//! the ribbon at some point, not because of any design intent (see
//! `ribbon.rs`, which still holds the shared `RibbonAction` enum). This
//! module now renders both halves in one bar:
//!
//! `[New] [Open] [Open Folder] [Save▾] [Find]   [B] [I] [</>] [Link]
//!  [H▾] [Bullet] [Numbered] [Quote] [TOC]   [Mermaid▾] ... [Export▾] [Terminal]`
//!
//! The formatting half (bold/italic/.../Mermaid) only renders for markdown
//! files; the file/tool half always renders. There is no collapsed state —
//! the bar is either shown (one view, one height) or not rendered at all
//! (Zen Mode, handled by the caller).
//!
//! # Quiet chrome
//!
//! The bar is deliberately recessive. It carries the *page's* background
//! rather than a panel tone, its controls rest at `text.muted` and only come
//! to full contrast under the pointer, and it groups by whitespace instead of
//! by rules — the seven hairline separators that used to slice it into
//! segments are gone. Nothing here is framed: the dropdowns are frameless
//! menu buttons cut from the same cloth as the icon buttons, so the eye reads
//! one strip of glyphs above the text rather than a row of form controls.

use crate::app::modifier_symbol;
use crate::markdown::formatting::{FormattingState, MarkdownFormatCommand};
use crate::markdown::mermaid::{mermaid_kind_menu_label, MermaidTemplateKind};
use crate::state::FileType;
use crate::theme::{accent, ThemeColors};
use crate::ui::phosphor_icons::{phosphor_font, CARET_DOWN, CARET_LEFT, CODE_BLOCK};
use crate::ui::skrivr_icons;
use crate::ui::RibbonAction;
use eframe::egui::{self, Color32, FontId, Ui, Vec2};
use egui_phosphor::regular::{
    CHECK, CLIPBOARD, EXPORT, FILE_MAGNIFYING_GLASS, FILE_PDF, FILE_PLUS, FILE_TEXT, FLOPPY_DISK,
    FOLDERS, FOLDER_SIMPLE_MINUS, GLOBE, LIGHTNING, MAGNIFYING_GLASS, PRINTER, SPARKLE,
    TERMINAL_WINDOW,
};
use rust_i18n::t;

/// Height of the merged toolbar.
///
/// Sized from the control, not the other way round: a 33 px button with the
/// same 4 px of air above and below that the 36 px bar gave its 28 px one.
/// Still well under the ~158 px of stacked chrome this bar was merged to
/// escape.
pub const TOOLBAR_HEIGHT: f32 = 41.0;

/// Footprint of a toolbar control. Comfortably past the 24 px WCAG 2.2 floor
/// (`a11y::MIN_TARGET_SIZE`), and sized to hold a 20 px glyph with the same
/// 13 px of surrounding air a 28 px button gave a 15 px one.
const BUTTON_SIZE: f32 = 33.0;

/// Corner radius of a control's hover/active fill. Scaled with the button so
/// the lozenge keeps its shape: 7 px on a 33 px square is the 6-on-28 curve.
const BUTTON_RADIUS: u8 = 7;

/// Gap between two groups of controls — the whole of the grouping signal now
/// that the separator rules are gone. Wide enough to parse as a break at a
/// glance, which a 1 px line at 7 px of padding never quite managed.
const GROUP_GAP: f32 = 14.0;

/// Gap between controls *within* a group. Tighter than the old flat 4 px:
/// the hover lozenges are what separate neighbours now, and a group whose
/// members nearly touch is what makes the gaps between groups legible.
const ITEM_GAP: f32 = 2.0;

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
    /// Point size, chosen so the two sets' glyph boxes come out the same
    /// height on screen.
    ///
    /// Raised from 11/13 to 15/16 and then to 20/21: at the smaller sizes the
    /// glyphs read as undersized against the body text they sit above, and
    /// 25/26 overshot it. Every control in the bar sizes its glyph from here
    /// — the file and tool buttons used to hardcode 14 and 13, which is why
    /// the strip never looked like one set.
    ///
    /// Skrivr is now the *smaller* of the two, which reverses what this used
    /// to assume. Measured on screen: Skrivr fits its artwork to an 860-unit
    /// box in a 1000-unit em (0.86 em of ink), where Phosphor's glyphs top
    /// out around 0.82 em. At an equal point size the Skrivr glyphs therefore
    /// render *larger*, not smaller — which is why `B` and `I` were visibly
    /// outgrowing the file icons beside them. 19 against 20 is that 0.82/0.86
    /// ratio.
    fn size(self) -> f32 {
        match self {
            IconGlyphFont::Phosphor => 20.0,
            IconGlyphFont::Skrivr => 19.0,
        }
    }

    /// How far below the layout box's centre this font's ink actually sits,
    /// as a fraction of the point size.
    ///
    /// `Align2::CENTER_CENTER` centres the *galley* — a box running from the
    /// font's ascent to its descent — not the ink inside it. Two fonts with
    /// different ascent/descent split their ink differently within that box,
    /// so centring both on the same point leaves one set riding higher than
    /// the other. Skrivr's ink landed 0.133 em above centre, which put the
    /// whole formatting half of the bar ~3 px above the file half.
    fn ink_nudge_em(self) -> f32 {
        match self {
            IconGlyphFont::Phosphor => 0.0,
            IconGlyphFont::Skrivr => 0.133,
        }
    }

    /// The point a glyph in this font should be centred on, given the centre
    /// of the control that holds it.
    fn ink_center(self, control_center: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            control_center.x,
            control_center.y + self.ink_nudge_em() * self.size(),
        )
    }

    fn font_id(self) -> FontId {
        match self {
            IconGlyphFont::Phosphor => phosphor_font(self.size()),
            IconGlyphFont::Skrivr => skrivr_icons::skrivr_font(self.size()),
        }
    }
}

/// Type for the `H` trigger, which is a letterform doing an icon's job.
///
/// It reads as one of the glyphs beside it rather than as a word, so it takes
/// its size from the icon scale — stepped down, because a cap-height letter
/// covers less of its point size than a Phosphor glyph does.
fn heading_label_font() -> FontId {
    FontId::proportional(IconGlyphFont::Phosphor.size() - 3.0)
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

        // The bar carries the page's own background, not a panel tone. A
        // secondary fill draws a horizontal band across the top of the window
        // and announces "chrome"; the paper colour lets the controls sit *on*
        // the document surface, which is the whole point of the quiet
        // direction. Separation is left to the tab strip's hairline below.
        let bar_bg = colors.base.background;

        let rect = ui.available_rect_before_wrap();
        let _response = ui.allocate_rect(rect, egui::Sense::hover());

        ui.painter().rect_filled(rect, 0.0, bar_bg);

        let mut button_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(Vec2::new(8.0, 4.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        button_ui.spacing_mut().item_spacing.x = ITEM_GAP;

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
        menu_icon_button(
            &mut button_ui,
            "toolbar_save_dropdown",
            FLOPPY_DISK,
            IconGlyphFont::Phosphor,
            &t!("menu.file.save").to_string(),
            true,
            colors,
            |ui| {
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
            },
        );

        // Structured data operations (JSON/YAML/TOML only)
        if file_type.is_structured() {
            button_ui.add_space(GROUP_GAP);

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

        button_ui.add_space(GROUP_GAP);

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
            button_ui.add_space(GROUP_GAP);

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

            button_ui.add_space(GROUP_GAP);

            // Heading dropdown
            let current_heading = formatting_state.and_then(|s| s.heading_level);
            let heading_label = current_heading
                .map(|h| format!("H{}", h as u8))
                .unwrap_or_else(|| "H".to_string());

            menu_text_button(
                &mut button_ui,
                "format_bar_heading_dropdown",
                &heading_label,
                // Sized a little under the icon point size, not at it: a
                // capital letter fills only its cap-height, where a Phosphor
                // glyph fills most of its em, so parity would leave the H
                // looking like the runt of the strip.
                heading_label_font(),
                &t!("format_toolbar.heading_menu_tooltip").to_string(),
                has_editor,
                // The trigger carries the caret's current heading level, so it
                // is a state readout as much as a control — it stays lit when
                // the cursor is inside a heading, the way the B and I buttons
                // do.
                current_heading.is_some(),
                colors,
                |ui| {
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
                },
            );

            button_ui.add_space(GROUP_GAP);

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

            button_ui.add_space(GROUP_GAP);

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

            button_ui.add_space(GROUP_GAP);

            // Mermaid diagram templates
            let mermaid_label_font = egui::TextStyle::Button.resolve(button_ui.style());
            menu_text_button(
                &mut button_ui,
                "format_bar_mermaid_dropdown",
                &t!("format_toolbar.mermaid_menu").to_string(),
                mermaid_label_font,
                &t!("format_toolbar.mermaid_entry_tooltip").to_string(),
                has_editor,
                false,
                colors,
                |ui| {
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
                },
            );
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
                ui.add_space(GROUP_GAP);

                menu_icon_button(
                    ui,
                    "toolbar_export_dropdown",
                    EXPORT,
                    IconGlyphFont::Phosphor,
                    &t!("ribbon.export_pdf").to_string(),
                    true,
                    colors,
                    |ui| {
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
                    },
                );
            }
        });

        FormatToolbarOutput { action }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared control painting
// ─────────────────────────────────────────────────────────────────────────────

/// Glyph colour for a toolbar control at rest, hovered, or disabled.
///
/// The quiet-chrome rule, in one place. A bar whose icons all sit at
/// `text.primary` reads as a row of equally loud controls competing with the
/// page below it; resting at `text.muted` lets the strip recede to texture,
/// and lifting to `text.primary` under the pointer makes each control
/// acknowledge the cursor before it is even pressed.
///
/// `text.muted` measures 5.24:1 (light) and 5.04:1 (dark) against the bar's
/// background — these are meaningful UI glyphs, so the floor that applies is
/// 3:1, and both themes clear it with room to spare. Guarded by
/// `theme::contrast_tests::toolbar_resting_glyph_meets_contrast_floor`.
fn glyph_color(colors: &ThemeColors, enabled: bool, hovered: bool) -> Color32 {
    if !enabled {
        colors.text.disabled
    } else if hovered {
        colors.text.primary
    } else {
        colors.text.muted
    }
}

/// Paint a control's hover/press/active surface, and return the colour its
/// glyph should be drawn in.
///
/// Every control in the bar goes through here, so the three feedback states
/// are the same treatment everywhere. They had drifted: the file and tool
/// buttons acknowledged a hover but not a press, and only the format buttons
/// had an active state at all.
fn paint_control_surface(
    ui: &Ui,
    btn: &egui::Response,
    colors: &ThemeColors,
    enabled: bool,
    active: bool,
) -> Color32 {
    let hovered = enabled && btn.hovered();
    // `.frame(false)` disables egui's own press feedback, so without an
    // explicit pressed state a click produces no acknowledgement at all until
    // the document changes. Pressed reads darker than hover so the two are
    // distinguishable while the pointer is still down.
    let pressed = enabled && btn.is_pointer_button_down_on();

    let fill = if pressed {
        Some(accent::lerp_color(colors.base.selected, Color32::BLACK, 0.12))
    } else if active {
        Some(colors.base.selected)
    } else if hovered {
        Some(colors.base.hover)
    } else {
        None
    };

    if let Some(fill) = fill {
        ui.painter()
            .rect_filled(btn.rect, egui::CornerRadius::same(BUTTON_RADIUS), fill);
    }

    if active || pressed {
        accent::on_accent(colors.base.selected)
    } else {
        glyph_color(colors, enabled, hovered)
    }
}

/// Allocate a frameless, square control of the bar's standard footprint.
///
/// The blank label is deliberate: the glyph is painted afterwards, centred in
/// the returned rect, because an icon font's own metrics do not centre it
/// optically inside a button.
fn allocate_control(ui: &mut Ui, enabled: bool, width: f32) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new("")
            .frame(false)
            .min_size(Vec2::new(width, BUTTON_SIZE)),
    )
}

/// Render a plain (non-toggling) file/tool icon button for the merged bar.
fn file_icon_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    colors: &ThemeColors,
) -> egui::Response {
    icon_button(ui, icon, IconGlyphFont::Phosphor, tooltip, enabled, false, colors)
}

/// Small icon button drawn from the bundled Skrivr set.
fn toolbar_icon_button(
    ui: &mut Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    colors: &ThemeColors,
) -> egui::Response {
    icon_button(ui, icon, IconGlyphFont::Skrivr, tooltip, enabled, false, colors)
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
    icon_button(ui, icon, icon_font, tooltip, enabled, active, colors)
}

/// The one icon control the bar is built from.
///
/// `file_icon_button`, `toolbar_icon_button` and `format_button` are now thin
/// names over this — they were three near-copies that had drifted to two
/// hover radii and three glyph sizes between them.
fn icon_button(
    ui: &mut Ui,
    icon: &str,
    icon_font: IconGlyphFont,
    tooltip: &str,
    enabled: bool,
    active: bool,
    colors: &ThemeColors,
) -> egui::Response {
    let btn = allocate_control(ui, enabled, BUTTON_SIZE);
    let glyph = paint_control_surface(ui, &btn, colors, enabled, active);

    ui.painter().text(
        icon_font.ink_center(btn.rect.center()),
        egui::Align2::CENTER_CENTER,
        icon,
        icon_font.font_id(),
        glyph,
    );

    if active {
        crate::ui::a11y::name_toggle(&btn, tooltip, enabled, active);
    } else {
        crate::ui::a11y::name_widget(&btn, tooltip, enabled);
    }
    crate::ui::a11y::focus_ring(ui, &btn, BUTTON_RADIUS);

    btn.on_hover_text(tooltip)
}

// ─────────────────────────────────────────────────────────────────────────────
// Menu controls
// ─────────────────────────────────────────────────────────────────────────────

/// Width of the caret that marks a control as opening a menu.
const CARET_WIDTH: f32 = 12.0;

/// Point size of that caret — small enough to read as a mark on the control
/// rather than as a second icon competing with the first. It grows far less
/// than the glyph it follows: at parity with the icon it would stop being a
/// footnote and start being a second icon.
const CARET_SIZE: f32 = 11.0;

/// Paint the down-caret that marks a menu trigger, at the right edge of its
/// rect.
///
/// Always one step quieter than the glyph beside it: the caret says "there is
/// more here", which is a footnote to the control's identity, not part of it.
fn paint_caret(ui: &Ui, rect: egui::Rect, glyph: Color32) {
    ui.painter().text(
        egui::pos2(rect.right() - CARET_WIDTH / 2.0 - 2.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        CARET_DOWN,
        phosphor_font(CARET_SIZE),
        glyph.gamma_multiply(0.75),
    );
}

/// An icon control that opens a menu.
///
/// Replaces a `ComboBox`, which egui frames as a bordered, fixed-width form
/// control — three of those among a row of frameless glyphs was the single
/// loudest thing in the old bar. This is the same button as its neighbours,
/// widened for a caret.
#[allow(clippy::too_many_arguments)]
fn menu_icon_button<R>(
    ui: &mut Ui,
    id_salt: &str,
    icon: &str,
    icon_font: IconGlyphFont,
    tooltip: &str,
    enabled: bool,
    colors: &ThemeColors,
    content: impl FnOnce(&mut Ui) -> R,
) {
    menu_control(
        ui,
        id_salt,
        BUTTON_SIZE + CARET_WIDTH,
        tooltip,
        enabled,
        false,
        colors,
        |ui, rect, glyph| {
            let center = icon_font
                .ink_center(egui::pos2(rect.center().x - CARET_WIDTH / 2.0, rect.center().y));
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                icon,
                icon_font.font_id(),
                glyph,
            );
        },
        content,
    );
}

/// A text-labelled control that opens a menu (the `H▾` and `Mermaid▾` ones).
///
/// The two want different type: `Mermaid▾` is a word and is set in the UI
/// font at the bar's own size (rather than the `ComboBox`'s hardcoded 11 px,
/// which rendered it a step smaller than every other string in the chrome),
/// while `H▾` is a letterform standing in for an icon and has to be sized
/// against the glyphs it sits between. Hence `font` rather than one fixed
/// text style.
#[allow(clippy::too_many_arguments)]
fn menu_text_button<R>(
    ui: &mut Ui,
    id_salt: &str,
    label: &str,
    font: FontId,
    tooltip: &str,
    enabled: bool,
    active: bool,
    colors: &ThemeColors,
    content: impl FnOnce(&mut Ui) -> R,
) {
    let label_width = ui
        .fonts_mut(|f| f.layout_no_wrap(label.to_owned(), font.clone(), Color32::WHITE))
        .size()
        .x;
    // 16 px of side padding keeps a one-character label ("H") from reading as
    // a cramped chip while a long one ("Mermaid…") still gets room.
    let width = (label_width + CARET_WIDTH + 16.0).max(BUTTON_SIZE + CARET_WIDTH);

    menu_control(
        ui,
        id_salt,
        width,
        tooltip,
        enabled,
        active,
        colors,
        move |ui, rect, glyph| {
            ui.painter().text(
                egui::pos2(rect.left() + 8.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                label,
                font.clone(),
                glyph,
            );
        },
        content,
    );
}

/// Shared body of the two menu controls: allocate, paint the surface, let the
/// caller draw its own content, add the caret, then hang a menu off it.
#[allow(clippy::too_many_arguments)]
fn menu_control<R>(
    ui: &mut Ui,
    id_salt: &str,
    width: f32,
    tooltip: &str,
    enabled: bool,
    active: bool,
    colors: &ThemeColors,
    paint_content: impl FnOnce(&Ui, egui::Rect, Color32),
    content: impl FnOnce(&mut Ui) -> R,
) {
    let button = egui::Button::new("")
        .frame(false)
        .min_size(Vec2::new(width, BUTTON_SIZE));

    let (btn, _inner) = egui::containers::menu::MenuButton::from_button(button)
        .ui(ui, |ui| content(ui));

    // An open menu keeps its trigger lit, so the pointer can leave the button
    // for the popup without the control appearing to switch off underneath it.
    let is_open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&btn));
    let glyph = paint_control_surface(ui, &btn, colors, enabled, active || is_open);

    paint_content(ui, btn.rect, glyph);
    paint_caret(ui, btn.rect, glyph);

    crate::ui::a11y::name_widget(&btn, tooltip, enabled);
    crate::ui::a11y::focus_ring(ui, &btn, BUTTON_RADIUS);

    let _ = btn.on_hover_text(tooltip);
    let _ = id_salt;
}

/// Output from the merged toolbar.
pub struct FormatToolbarOutput {
    /// Action triggered by a button click.
    pub action: Option<RibbonAction>,
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
