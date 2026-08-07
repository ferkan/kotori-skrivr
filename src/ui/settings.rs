//! Settings Panel Component for Ferrite
//!
//! This module implements a modal settings panel that allows users to configure
//! appearance, editor behavior, and file handling options with live preview.

use crate::config::{
    CjkFontPreference, EditorFont, HeaderSpacing, KeyBinding, KeyCode, KeyModifiers,
    KeyboardShortcuts, Language, MaxLineWidth, MinimapMode, Settings, ShortcutCommand, Theme,
    ViewMode,
};
use crate::fonts;
use crate::markdown::syntax::get_available_themes;
use crate::terminal::MonitorInfo;
use crate::theme::typescale::chrome;
use crate::ui::icons::phosphor_rich_text;
use crate::ui::phosphor_icons::{
    ARROWS_COUNTER_CLOCKWISE, CHECK, CONFETTI, DESKTOP, GEAR, GLOBE, MAGNIFYING_GLASS, MOON, SUN,
    WARNING, X,
};
use crate::ui::settings_layout as layout;
use crate::update::{self, UpdateCheckResult, UpdateState};
use eframe::egui::{self, Color32, RichText, Ui};
use rust_i18n::{set_locale, t};
use std::sync::mpsc;

// ─────────────────────────────────────────────────────────────────────────────
// Localized Display Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Get localized font description
fn font_description(font: &EditorFont) -> String {
    match font {
        // No localization key yet for this new default font; fall back to the
        // built-in English description rather than adding i18n assets, which
        // is out of scope for this change.
        EditorFont::Literata => font.description().to_string(),
        EditorFont::Inter => t!("settings.editor.font_inter_desc").to_string(),
        EditorFont::JetBrainsMono => t!("settings.editor.font_jetbrains_desc").to_string(),
        EditorFont::Custom(_) => t!("settings.editor.custom_font_desc").to_string(),
    }
}

/// Get localized view mode description
fn view_mode_description(mode: &ViewMode) -> String {
    match mode {
        ViewMode::Raw => t!("view_mode.raw_desc").to_string(),
        ViewMode::Rendered => t!("view_mode.rendered_desc").to_string(),
        ViewMode::Split => t!("view_mode.split_desc").to_string(),
        // No localization key yet for this new mode; fall back to the
        // built-in English description rather than adding i18n assets,
        // which is out of scope for this change.
        ViewMode::LiveMarkdown => mode.description().to_string(),
    }
}

/// Get localized shortcut command name
fn shortcut_command_name(cmd: &ShortcutCommand) -> String {
    match cmd {
        // File operations
        ShortcutCommand::Save => t!("shortcuts.commands.save").to_string(),
        ShortcutCommand::SaveAs => t!("shortcuts.commands.save_as").to_string(),
        ShortcutCommand::Open => t!("shortcuts.commands.open").to_string(),
        ShortcutCommand::New => t!("shortcuts.commands.new").to_string(),
        ShortcutCommand::NewTab => t!("shortcuts.commands.new_tab").to_string(),
        ShortcutCommand::CloseTab => t!("shortcuts.commands.close_tab").to_string(),
        // Navigation
        ShortcutCommand::NextTab => t!("shortcuts.commands.next_tab").to_string(),
        ShortcutCommand::PrevTab => t!("shortcuts.commands.prev_tab").to_string(),
        ShortcutCommand::GoToLine => t!("shortcuts.commands.go_to_line").to_string(),
        ShortcutCommand::QuickOpen => t!("shortcuts.commands.quick_open").to_string(),
        // View
        ShortcutCommand::ToggleViewMode => t!("shortcuts.commands.toggle_view_mode").to_string(),
        ShortcutCommand::CycleTheme => t!("shortcuts.commands.cycle_theme").to_string(),
        ShortcutCommand::ToggleZenMode => t!("shortcuts.commands.toggle_zen_mode").to_string(),
        ShortcutCommand::ToggleFullscreen => t!("shortcuts.commands.toggle_fullscreen").to_string(),
        ShortcutCommand::ToggleOutline => t!("shortcuts.commands.toggle_outline").to_string(),
        ShortcutCommand::ToggleFileTree => t!("shortcuts.commands.toggle_file_tree").to_string(),
        ShortcutCommand::TogglePipeline => t!("shortcuts.commands.toggle_pipeline").to_string(),
        // Edit
        ShortcutCommand::Undo => t!("shortcuts.commands.undo").to_string(),
        ShortcutCommand::Redo => t!("shortcuts.commands.redo").to_string(),
        ShortcutCommand::DeleteLine => t!("shortcuts.commands.delete_line").to_string(),
        ShortcutCommand::DuplicateLine => t!("shortcuts.commands.duplicate_line").to_string(),
        ShortcutCommand::MoveLineUp => t!("shortcuts.commands.move_line_up").to_string(),
        ShortcutCommand::MoveLineDown => t!("shortcuts.commands.move_line_down").to_string(),
        ShortcutCommand::SelectNextOccurrence => {
            t!("shortcuts.commands.select_next_occurrence").to_string()
        }
        // Search
        ShortcutCommand::Find => t!("shortcuts.commands.find").to_string(),
        ShortcutCommand::FindReplace => t!("shortcuts.commands.find_replace").to_string(),
        ShortcutCommand::FindNext => t!("shortcuts.commands.find_next").to_string(),
        ShortcutCommand::FindPrev => t!("shortcuts.commands.find_prev").to_string(),
        ShortcutCommand::SearchInFiles => t!("shortcuts.commands.search_in_files").to_string(),
        // Formatting
        ShortcutCommand::FormatBold => t!("shortcuts.commands.bold").to_string(),
        ShortcutCommand::FormatItalic => t!("shortcuts.commands.italic").to_string(),
        ShortcutCommand::FormatInlineCode => t!("shortcuts.commands.inline_code").to_string(),
        ShortcutCommand::FormatCodeBlock => t!("shortcuts.commands.code_block").to_string(),
        ShortcutCommand::FormatLink => t!("shortcuts.commands.link").to_string(),
        ShortcutCommand::FormatImage => t!("shortcuts.commands.image").to_string(),
        ShortcutCommand::FormatBlockquote => t!("shortcuts.commands.blockquote").to_string(),
        ShortcutCommand::FormatBulletList => t!("shortcuts.commands.bullet_list").to_string(),
        ShortcutCommand::FormatNumberedList => t!("shortcuts.commands.numbered_list").to_string(),
        ShortcutCommand::FormatHeading1 => t!("shortcuts.commands.heading_1").to_string(),
        ShortcutCommand::FormatHeading2 => t!("shortcuts.commands.heading_2").to_string(),
        ShortcutCommand::FormatHeading3 => t!("shortcuts.commands.heading_3").to_string(),
        ShortcutCommand::FormatHeading4 => t!("shortcuts.commands.heading_4").to_string(),
        ShortcutCommand::FormatHeading5 => t!("shortcuts.commands.heading_5").to_string(),
        ShortcutCommand::FormatHeading6 => t!("shortcuts.commands.heading_6").to_string(),
        // Folding
        ShortcutCommand::FoldAll => t!("shortcuts.commands.fold_all").to_string(),
        ShortcutCommand::UnfoldAll => t!("shortcuts.commands.unfold_all").to_string(),
        ShortcutCommand::ToggleFoldAtCursor => t!("shortcuts.commands.toggle_fold").to_string(),
        // Other
        ShortcutCommand::OpenSettings => t!("shortcuts.commands.open_settings").to_string(),
        ShortcutCommand::OpenAbout => t!("shortcuts.commands.open_about").to_string(),
        ShortcutCommand::ExportHtml => t!("shortcuts.commands.export_html").to_string(),
        ShortcutCommand::ExportPdf => t!("shortcuts.commands.export_pdf").to_string(),
        ShortcutCommand::PrintPreview => t!("shortcuts.commands.print_preview").to_string(),
        ShortcutCommand::InsertToc => t!("shortcuts.commands.insert_toc").to_string(),
        ShortcutCommand::ToggleTerminal => cmd.display_name().to_string(),
        ShortcutCommand::ToggleProductivityHub => cmd.display_name().to_string(),
        ShortcutCommand::ToggleFrontmatter => "Toggle Frontmatter Panel".to_string(),
        ShortcutCommand::ZoomIn => "Zoom In".to_string(),
        ShortcutCommand::ZoomOut => "Zoom Out".to_string(),
        ShortcutCommand::ResetZoom => "Reset Zoom".to_string(),
        ShortcutCommand::CommandPalette => "Command Palette".to_string(),
        ShortcutCommand::OpenWorkspace => "Open Workspace".to_string(),
        ShortcutCommand::CloseWorkspace => "Close Workspace".to_string(),
    }
}

/// Settings panel sections for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    Appearance,
    Editor,
    Files,
    Keyboard,
    Terminal,
    About,
}

impl SettingsSection {
    /// Get the display label for the section.
    pub fn label(&self) -> String {
        match self {
            SettingsSection::Appearance => t!("settings.appearance.title"),
            SettingsSection::Editor => t!("settings.editor.title"),
            SettingsSection::Files => t!("settings.files.title"),
            SettingsSection::Keyboard => t!("settings.keyboard.title"),
            SettingsSection::Terminal => t!("settings.terminal.title"),
            SettingsSection::About => t!("settings.about.title"),
        }
        .to_string()
    }

    /// Get the icon for the section.
    pub fn icon(&self) -> &'static str {
        use crate::ui::phosphor_icons::{
            FILES, INFO, KEYBOARD, NOTE_PENCIL, PALETTE, TERMINAL_WINDOW,
        };
        match self {
            SettingsSection::Appearance => PALETTE,
            SettingsSection::Editor => NOTE_PENCIL,
            SettingsSection::Files => FILES,
            SettingsSection::Keyboard => KEYBOARD,
            SettingsSection::Terminal => TERMINAL_WINDOW,
            SettingsSection::About => INFO,
        }
    }
}

/// Result of showing the settings panel.
#[derive(Debug, Clone, Default)]
pub struct SettingsPanelOutput {
    /// Whether settings were modified.
    pub changed: bool,
    /// Whether a reset to defaults was requested.
    pub reset_requested: bool,
}

/// State for capturing a new key binding.
#[derive(Debug, Clone)]
pub struct KeyCaptureState {
    /// Which command is being rebound
    pub command: ShortcutCommand,
    /// Captured modifiers so far
    pub modifiers: KeyModifiers,
    /// Captured key (if any)
    pub key: Option<KeyCode>,
}

/// Settings panel state and rendering.
#[derive(Debug)]
pub struct SettingsPanel {
    /// Currently active settings section.
    active_section: SettingsSection,
    /// State for capturing a new key binding (None if not capturing)
    key_capture: Option<KeyCaptureState>,
    /// Filter text for keyboard shortcuts search
    keyboard_filter: String,
    /// Conflict warning message (if any)
    conflict_warning: Option<(ShortcutCommand, String)>,
    /// Cached monitor info
    cached_monitor_info: Option<Vec<MonitorInfo>>,
    /// Current update check state
    update_state: UpdateState,
    /// Receiver for background update check result
    update_check_rx: Option<mpsc::Receiver<UpdateCheckResult>>,
}

impl Default for SettingsPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPanel {
    /// Create a new settings panel instance.
    pub fn new() -> Self {
        Self {
            active_section: SettingsSection::default(),
            key_capture: None,
            keyboard_filter: String::new(),
            conflict_warning: None,
            cached_monitor_info: None,
            update_state: UpdateState::default(),
            update_check_rx: None,
        }
    }

    /// Render the settings panel inline within a tab (not as a modal window).
    ///
    /// This is used when settings are displayed as a special tab in the main
    /// editor area, giving more screen real estate than the modal version.
    pub fn show_inline(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        is_dark: bool,
        workspace_root: Option<&std::path::Path>,
    ) -> SettingsPanelOutput {
        let mut output = SettingsPanelOutput::default();

        // House control metrics for the whole panel: combo padding so a
        // selected value is not flush against its own border, and one slider
        // width. Set centrally so a new control inherits the style rather than
        // having to remember it.
        layout::apply_control_style(ui);

        let available = ui.available_size();
        let sidebar_width = 160.0;

        ui.horizontal(|ui| {
            // Left side: Section tabs
            ui.vertical(|ui| {
                ui.set_min_width(sidebar_width);
                ui.set_max_width(sidebar_width);
                ui.set_min_height(available.y - 50.0);

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    // Heading and category buttons share one left inset so they
                    // cannot drift apart; the heading previously sat flush
                    // against the panel edge.
                    ui.add_space(layout::SIDEBAR_PADDING_X);
                    ui.label(phosphor_rich_text(GEAR, chrome::SECTION));
                    ui.label(
                        RichText::new(t!("settings.title"))
                            .font(crate::fonts::chrome_bold_font(chrome::TITLE)),
                    );
                });
                ui.add_space(14.0);

                for section in [
                    SettingsSection::Appearance,
                    SettingsSection::Editor,
                    SettingsSection::Files,
                    SettingsSection::Keyboard,
                    SettingsSection::Terminal,
                    SettingsSection::About,
                ] {
                    let selected = self.active_section == section;
                    let text = format!("{} {}", section.icon(), section.label());

                    // Left-align the label. egui centres a button's atoms in an
                    // enlarged rect, which left the category names floating in
                    // the middle of the column. `right_text("")` appends a
                    // growing spacer on the right, pushing the label left —
                    // the mechanism `left_text`/`right_text` are built on.
                    let btn = ui
                        .scope(|ui| {
                            ui.spacing_mut().button_padding =
                                egui::vec2(layout::SIDEBAR_PADDING_X, 7.0);
                            ui.add_sized(
                                [sidebar_width - layout::SIDEBAR_PADDING_X, 34.0],
                                egui::Button::selectable(
                                    selected,
                                    RichText::new(text).size(chrome::BODY),
                                )
                                .right_text(""),
                            )
                        })
                        .inner;

                    if btn.clicked() {
                        self.active_section = section;
                        if section != SettingsSection::Keyboard {
                            self.key_capture = None;
                            self.conflict_warning = None;
                        }
                    }
                }

                // Reset button at the bottom of sidebar
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                if ui
                    .add_sized(
                        [sidebar_width - 16.0, 28.0],
                        egui::Button::new(format!(
                            "{} {}",
                            ARROWS_COUNTER_CLOCKWISE,
                            t!("settings.reset_all")
                        )),
                    )
                    .on_hover_text(t!("settings.reset_tooltip"))
                    .clicked()
                {
                    output.reset_requested = true;
                }
            });

            ui.separator();

            // Right side: Section content (fills remaining space)
            ui.vertical(|ui| {
                let content_width = (available.x - sidebar_width - 24.0).max(300.0);
                ui.set_min_width(content_width);

                egui::ScrollArea::vertical()
                    .id_salt(format!("settings_inline_scroll_{:?}", self.active_section))
                    .show(ui, |ui| {
                        ui.set_min_width(content_width - 16.0);
                        ui.add_space(8.0);

                        match self.active_section {
                            SettingsSection::Appearance => {
                                if self.show_appearance_section(ui, settings, is_dark) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::Editor => {
                                if self.show_editor_section(ui, settings, workspace_root) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::Files => {
                                if self.show_files_section(ui, settings) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::Keyboard => {
                                if self.show_keyboard_section(ui, settings) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::Terminal => {
                                if self.show_terminal_section(ui, settings) {
                                    output.changed = true;
                                }
                            }
                            SettingsSection::About => {
                                let ctx = ui.ctx().clone();
                                self.show_about_section(ui, &ctx);
                            }
                        }
                    });
            });
        });

        output
    }

    /// Show the Terminal settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_terminal_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        layout::section_heading(ui, t!("settings.terminal.title").to_string());

        // Terminal Enabled
        if ui
            .checkbox(
                &mut settings.terminal_enabled,
                t!("settings.terminal.enable").to_string(),
            )
            .changed()
        {
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Terminal Font Size
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.terminal.font_size").to_string()).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            ui.label(format!("{}px", settings.terminal_font_size as u32));
        });
        ui.add_space(4.0);

        ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
        let font_slider = ui.add(
            egui::Slider::new(&mut settings.terminal_font_size, 10.0..=32.0)
                .show_value(false)
                .step_by(1.0),
        );
        if font_slider.changed() {
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Scrollback Lines
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.terminal.scrollback").to_string()).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            ui.label(format!("{}", settings.terminal_scrollback_lines));
        });
        ui.add_space(4.0);

        let mut scrollback_val = settings.terminal_scrollback_lines as f64;
        ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
        let scrollback_slider = ui.add(
            egui::Slider::new(&mut scrollback_val, 1000.0..=50000.0)
                .show_value(false)
                .step_by(1000.0),
        );
        if scrollback_slider.changed() {
            settings.terminal_scrollback_lines = scrollback_val as usize;
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Copy on Select
        if ui
            .checkbox(
                &mut settings.terminal_copy_on_select,
                t!("settings.terminal.copy_selection").to_string(),
            )
            .on_hover_text(t!("settings.terminal.copy_selection_tooltip").to_string())
            .changed()
        {
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Terminal Theme
        layout::section_heading(ui, t!("settings.terminal.theme").to_string());

        egui::ComboBox::from_id_salt("terminal_theme_combo")
            .selected_text(&settings.terminal_theme_name)
            .width(layout::CONTROL_WIDTH)
            .show_ui(ui, |ui| {
                for theme in crate::terminal::TerminalTheme::all() {
                    if ui
                        .selectable_value(
                            &mut settings.terminal_theme_name,
                            theme.name.clone(),
                            &theme.name,
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Terminal Opacity
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.terminal.opacity").to_string()).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            ui.label(format!("{:.0}%", settings.terminal_opacity * 100.0));
        });
        ui.add_space(4.0);

        ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
        if ui
            .add(egui::Slider::new(&mut settings.terminal_opacity, 0.1..=1.0).show_value(false))
            .changed()
        {
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Terminal Startup Command
        layout::section_heading(ui, t!("settings.terminal.startup_command").to_string());
        layout::description(ui, t!("settings.terminal.startup_command_desc").to_string());
        ui.add_space(4.0);

        if ui
            .add(
                egui::TextEdit::singleline(&mut settings.terminal_startup_command)
                    .hint_text(t!("settings.terminal.startup_command_hint").to_string()),
            )
            .changed()
        {
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Monitor Information
        layout::section_heading(ui, t!("settings.terminal.monitors").to_string());
        layout::description(ui, t!("settings.terminal.monitors_desc").to_string());
        ui.add_space(4.0);

        if self.cached_monitor_info.is_none() {
            self.cached_monitor_info = Some(crate::terminal::detect_monitors());
        }
        let monitors = self.cached_monitor_info.as_ref().unwrap();

        egui::Frame::NONE
            .fill(ui.visuals().faint_bg_color)
            .corner_radius(4.0)
            .inner_margin(8.0)
            .show(ui, |ui| {
                for (i, m) in monitors.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(t!("settings.terminal.monitor_label", index = i + 1).to_string());
                        ui.label(RichText::new(&m.name).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
                        ui.label(
                            t!(
                                "settings.terminal.monitor_geometry",
                                width = m.width as u32,
                                height = m.height as u32,
                                x = m.x as i32,
                                y = m.y as i32
                            )
                            .to_string(),
                        );
                    });
                }
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Breathing color
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.terminal.breathing_color").to_string()).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            if ui
                .color_edit_button_srgba(&mut settings.terminal_breathing_color)
                .changed()
            {
                changed = true;
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Prompt patterns
        layout::section_heading(ui, t!("settings.terminal.prompt_patterns").to_string());
        layout::description(ui, t!("settings.terminal.prompt_patterns_desc").to_string());
        ui.add_space(4.0);

        let mut patterns_text = settings.terminal_prompt_patterns.join("\n");
        if ui
            .add(
                egui::TextEdit::multiline(&mut patterns_text)
                    .desired_rows(3)
                    .hint_text(t!("settings.terminal.prompt_patterns_hint").to_string()),
            )
            .changed()
        {
            settings.terminal_prompt_patterns = patterns_text
                .lines()
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .collect();
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Auto-load
        if ui
            .checkbox(
                &mut settings.terminal_auto_load_layout,
                t!("settings.terminal.auto_load_layout").to_string(),
            )
            .on_hover_text(t!("settings.terminal.auto_load_layout_tooltip").to_string())
            .changed()
        {
            changed = true;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Sound Notification
        layout::section_heading(ui, t!("settings.terminal.sound_notification").to_string());
        layout::description(ui, t!("settings.terminal.sound_notification_desc").to_string());
        ui.add_space(4.0);

        if ui
            .checkbox(
                &mut settings.terminal_sound_enabled,
                t!("settings.terminal.enable_sound").to_string(),
            )
            .on_hover_text(t!("settings.terminal.enable_sound_tooltip").to_string())
            .changed()
        {
            changed = true;
        }

        if settings.terminal_sound_enabled {
            ui.add_space(4.0);
            ui.indent("sound_file_settings", |ui| {
                ui.label(RichText::new(t!("settings.terminal.custom_sound").to_string()).small());
                let mut sound_path = settings.terminal_sound_file.clone().unwrap_or_default();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut sound_path)
                            .hint_text(t!("settings.terminal.custom_sound_hint").to_string()),
                    )
                    .changed()
                {
                    settings.terminal_sound_file = if sound_path.is_empty() {
                        None
                    } else {
                        Some(sound_path)
                    };
                    changed = true;
                }
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Focus on Detect
        layout::section_heading(ui, t!("settings.terminal.auto_focus").to_string());
        layout::description(ui, t!("settings.terminal.auto_focus_desc").to_string());
        ui.add_space(4.0);

        if ui
            .checkbox(
                &mut settings.terminal_focus_on_detect,
                t!("settings.terminal.focus_on_prompt").to_string(),
            )
            .on_hover_text(t!("settings.terminal.focus_on_prompt_tooltip").to_string())
            .changed()
        {
            changed = true;
        }

        changed
    }

    /// Show the About section with version info and update check.
    fn show_about_section(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        // Poll for update check result if we have a pending check
        if let Some(rx) = &self.update_check_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    UpdateCheckResult::UpToDate => {
                        self.update_state = UpdateState::UpToDate;
                    }
                    UpdateCheckResult::UpdateAvailable {
                        version,
                        release_url,
                        ..
                    } => {
                        self.update_state = UpdateState::UpdateAvailable {
                            version,
                            release_url,
                        };
                    }
                    UpdateCheckResult::Error(msg) => {
                        self.update_state = UpdateState::Error(msg);
                    }
                }
                self.update_check_rx = None;
            }
        }

        // Request repaint while checking so we poll the channel
        if matches!(self.update_state, UpdateState::Checking) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        layout::section_heading(ui, t!("settings.about.title").to_string());

        // Application info
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::branding::APP_NAME)
                    .font(crate::fonts::chrome_bold_font(chrome::BODY)),
            );
            ui.label(
                RichText::new(format!("v{}", update::current_version()))
                    .monospace()
                    .size(chrome::SMALL),
            );
        });
        ui.add_space(4.0);
        layout::description(ui, t!("settings.about.description").to_string());

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Check for Updates section
        layout::section_heading(ui, t!("settings.about.updates").to_string());

        match &self.update_state {
            UpdateState::Idle => {
                if ui
                    .button(t!("settings.about.check_for_updates").to_string())
                    .clicked()
                {
                    self.update_state = UpdateState::Checking;
                    self.update_check_rx = Some(update::spawn_update_check());
                }
            }
            UpdateState::Checking => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(t!("settings.about.checking"));
                });
            }
            UpdateState::UpToDate => {
                let success_color = if ui.visuals().dark_mode {
                    Color32::from_rgb(75, 210, 100)
                } else {
                    Color32::from_rgb(40, 167, 69)
                };
                ui.horizontal(|ui| {
                    ui.label(phosphor_rich_text(CHECK, 14.0).color(success_color));
                    ui.label(
                        RichText::new(t!("settings.about.up_to_date").to_string())
                            .color(success_color),
                    );
                });
                ui.add_space(8.0);
                if ui.small_button(t!("settings.about.check_again")).clicked() {
                    self.update_state = UpdateState::Checking;
                    self.update_check_rx = Some(update::spawn_update_check());
                }
            }
            UpdateState::UpdateAvailable {
                version,
                release_url,
            } => {
                let version = version.clone();
                let url = release_url.clone();

                egui::Frame::NONE
                    .fill(ui.visuals().faint_bg_color)
                    .corner_radius(6.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(phosphor_rich_text(CONFETTI, 14.0).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
                            ui.label(
                                RichText::new(t!("settings.about.update_available").to_string())
                                    .font(crate::fonts::chrome_bold_font(chrome::BODY)),
                            );
                        });
                        ui.add_space(4.0);
                        ui.label(format!(
                            "{}: v{} → v{}",
                            t!("settings.about.new_version"),
                            update::current_version(),
                            version
                        ));
                        ui.add_space(8.0);
                        if ui
                            .button(t!("settings.about.view_release").to_string())
                            .clicked()
                        {
                            let _ = open::that(&url);
                        }
                    });
                ui.add_space(8.0);
                if ui.small_button(t!("settings.about.check_again")).clicked() {
                    self.update_state = UpdateState::Checking;
                    self.update_check_rx = Some(update::spawn_update_check());
                }
            }
            UpdateState::Error(msg) => {
                ui.horizontal(|ui| {
                    ui.label(phosphor_rich_text(WARNING, 14.0).color(ui.visuals().error_fg_color));
                    ui.label(
                        RichText::new(t!("settings.about.check_failed").to_string())
                            .color(ui.visuals().error_fg_color),
                    );
                });
                ui.label(RichText::new(msg).small().weak());
                ui.add_space(8.0);
                if ui
                    .button(t!("settings.about.try_again").to_string())
                    .clicked()
                {
                    self.update_state = UpdateState::Checking;
                    self.update_check_rx = Some(update::spawn_update_check());
                }
            }
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Links
        layout::section_heading(ui, t!("settings.about.links").to_string());

        if ui
            .link(t!("settings.about.all_releases").to_string())
            .clicked()
        {
            let _ = open::that("https://github.com/OlaProeis/Ferrite/releases");
        }
        if ui
            .link(format!("🐛 {}", t!("settings.about.report_issue")))
            .clicked()
        {
            let _ = open::that("https://github.com/OlaProeis/Ferrite/issues");
        }
        if ui
            .link(format!("📖 {}", t!("settings.about.source_code")))
            .clicked()
        {
            let _ = open::that("https://github.com/OlaProeis/Ferrite");
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // License
        ui.label(RichText::new(t!("settings.about.license")).small().weak());
    }

    /// Show the Appearance settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_appearance_section(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        _is_dark: bool,
    ) -> bool {
        let mut changed = false;

        layout::section_heading(ui, t!("settings.appearance.title").to_string());

        // Theme selection
        layout::section_heading(ui, t!("settings.general.theme").to_string());

        ui.horizontal(|ui| {
            for theme in [Theme::Light, Theme::Dark, Theme::System] {
                let label = match theme {
                    Theme::Light => format!("{} {}", SUN, t!("settings.general.theme_light")),
                    Theme::Dark => format!("{} {}", MOON, t!("settings.general.theme_dark")),
                    Theme::System => format!("{} {}", DESKTOP, t!("settings.general.theme_system")),
                };
                if ui
                    .selectable_value(&mut settings.theme, theme, label)
                    .changed()
                {
                    changed = true;
                }
            }
        });

        layout::section_heading(ui, t!("settings.appearance.accent_color").to_string());
        layout::description(ui, t!("settings.appearance.accent_color_hint").to_string());
        ui.horizontal(|ui| {
            let mut c = egui::Color32::from_rgb(
                settings.accent_color[0],
                settings.accent_color[1],
                settings.accent_color[2],
            );
            if ui.color_edit_button_srgba(&mut c).changed() {
                settings.accent_color = [c.r(), c.g(), c.b()];
                changed = true;
            }
            ui.add_space(8.0);
            if ui
                .small_button(t!("settings.appearance.accent_reset"))
                .clicked()
            {
                settings.accent_color = crate::theme::accent::DEFAULT_ACCENT_RGB;
                changed = true;
            }
        });

        // Syntax highlighting theme
        layout::section_heading(ui, t!("settings.appearance.syntax_theme").to_string());
        layout::description(ui, t!("settings.appearance.syntax_theme_hint").to_string());
        ui.add_space(4.0);

        let themes = get_available_themes();
        let current_display = if settings.syntax_theme.is_empty() {
            t!("settings.appearance.syntax_theme_auto").to_string()
        } else {
            themes
                .iter()
                .find(|(name, _)| name == &settings.syntax_theme)
                .map(|(_, display)| display.clone())
                .unwrap_or_else(|| settings.syntax_theme.clone())
        };

        egui::ComboBox::from_id_salt("syntax_theme_combo")
            .selected_text(&current_display)
            .width(layout::CONTROL_WIDTH)
            .show_ui(ui, |ui| {
                ui.set_min_width(layout::CONTROL_WIDTH);
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        // Auto option (empty string = use dark/light default)
                        if ui
                            .selectable_label(
                                settings.syntax_theme.is_empty(),
                                t!("settings.appearance.syntax_theme_auto"),
                            )
                            .clicked()
                        {
                            settings.syntax_theme = String::new();
                            changed = true;
                        }

                        ui.separator();

                        // List all available themes
                        for (theme_name, display_name) in &themes {
                            if ui
                                .selectable_label(
                                    &settings.syntax_theme == theme_name,
                                    display_name,
                                )
                                .clicked()
                            {
                                settings.syntax_theme = theme_name.clone();
                                changed = true;
                            }
                        }
                    });
            });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Language selection
        layout::section_heading(ui, t!("settings.appearance.language").to_string());
        layout::description(ui, t!("settings.appearance.language_hint").to_string());
        ui.add_space(4.0);

        let current_lang = settings.language;
        egui::ComboBox::from_id_salt("language_combo")
            .selected_text(format!(
                "{} {}",
                GLOBE,
                current_lang.selector_display_name()
            ))
            .width(layout::CONTROL_WIDTH)
            .show_ui(ui, |ui| {
                for lang in Language::all() {
                    if ui
                        .selectable_value(
                            &mut settings.language,
                            *lang,
                            lang.selector_display_name(),
                        )
                        .changed()
                    {
                        // Apply language change immediately
                        set_locale(settings.language.locale_code());
                        changed = true;
                    }
                }
            });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Default View Mode selection
        layout::section_heading(ui, t!("settings.preview.default_view").to_string());
        layout::description(ui, t!("settings.default_view_hint").to_string());
        ui.add_space(4.0);

        for view_mode in ViewMode::all() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(
                        &mut settings.default_view_mode,
                        *view_mode,
                        format!("{} {}", view_mode.icon(), view_mode.label()),
                    )
                    .changed()
                {
                    changed = true;
                }
                ui.label(
                    RichText::new(view_mode_description(view_mode))
                        .weak()
                        .small(),
                );
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Font family selection
        layout::section_heading(ui, t!("settings.editor.font_family").to_string());

        // Built-in fonts
        for font in EditorFont::builtin_fonts() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(&mut settings.font_family, font.clone(), font.display_name())
                    .changed()
                {
                    changed = true;
                }
                ui.label(RichText::new(font_description(font)).weak().small());
            });
        }

        // Custom font option
        let is_custom = settings.font_family.is_custom();
        let custom_label = t!("settings.editor.custom_font");
        ui.horizontal(|ui| {
            if ui
                .selectable_label(is_custom, custom_label.to_string())
                .clicked()
                && !is_custom
            {
                // Defer loading until the user picks a family — the first sorted name from
                // enumeration may not resolve via font-kit on some macOS setups (GitHub #133).
                settings.font_family = EditorFont::Custom(String::new());
                changed = true;
            }
            ui.label(
                RichText::new(t!("settings.editor.custom_font_desc"))
                    .weak()
                    .small(),
            );
        });

        // Show system font picker when Custom is selected
        if let EditorFont::Custom(current_font) = &settings.font_family {
            // Clone the current font name to avoid borrow conflicts
            let current_font_name = current_font.clone();
            let system_fonts = fonts::list_system_fonts();
            let font_found = !current_font_name.trim().is_empty()
                && system_fonts.iter().any(|f| f == &current_font_name);

            ui.add_space(4.0);
            ui.indent("custom_font_picker", |ui| {
                ui.label(RichText::new(t!("settings.editor.select_system_font")).small());
                ui.add_space(2.0);

                let combo_label = if current_font_name.trim().is_empty() {
                    t!("settings.editor.custom_font_pick_placeholder").to_string()
                } else {
                    current_font_name.clone()
                };

                egui::ComboBox::from_id_salt("system_font_combo")
                    .selected_text(combo_label)
                    .width(layout::CONTROL_WIDTH)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(layout::CONTROL_WIDTH);
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for font_name in system_fonts {
                                    if ui
                                        .selectable_label(
                                            font_name == &current_font_name,
                                            font_name,
                                        )
                                        .clicked()
                                    {
                                        settings.font_family =
                                            EditorFont::Custom(font_name.to_string());
                                        changed = true;
                                    }
                                }
                            });
                    });

                // Font preview
                ui.add_space(4.0);
                ui.label(RichText::new(t!("settings.editor.font_preview")).small());
                ui.label(
                    RichText::new("The quick brown fox jumps over the lazy dog. 0123456789")
                        .size(chrome::BODY),
                );
                if !current_font_name.trim().is_empty() && !font_found {
                    ui.label(
                        RichText::new(t!("settings.editor.font_not_found"))
                            .color(ui.visuals().error_fg_color)
                            .small(),
                    );
                }
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Font size slider
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.font_size")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            ui.label(format!("{}px", settings.font_size as u32));
        });
        ui.add_space(4.0);

        ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
        let font_slider = ui.add(
            egui::Slider::new(
                &mut settings.font_size,
                Settings::MIN_FONT_SIZE..=Settings::MAX_FONT_SIZE,
            )
            .show_value(false)
            .step_by(1.0),
        );
        if font_slider.changed() {
            changed = true;
        }

        // Font size presets
        ui.horizontal(|ui| {
            for (label, size) in [
                (t!("settings.font_size.small"), 12.0),
                (t!("settings.font_size.medium"), 14.0),
                (t!("settings.font_size.large"), 18.0),
            ] {
                if ui.small_button(label).clicked() {
                    settings.font_size = size;
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Additional Scripts (complex script font preferences)
        layout::section_heading(ui, t!("settings.editor.complex_scripts").to_string());
        layout::description(ui, t!("settings.editor.complex_scripts_hint").to_string());
        ui.add_space(4.0);

        const COMPLEX_SCRIPTS: &[(&str, &str)] = &[
            ("arabic", "Arabic"),
            ("bengali", "Bengali"),
            ("devanagari", "Devanagari"),
            ("thai", "Thai"),
            ("hebrew", "Hebrew"),
            ("tamil", "Tamil"),
            ("georgian", "Georgian"),
            ("armenian", "Armenian"),
            ("ethiopic", "Ethiopic"),
            ("other_indic", "Other Indic"),
            ("southeast_asian", "Southeast Asian"),
        ];

        for (index, (key, display_name)) in COMPLEX_SCRIPTS.iter().enumerate() {
            let current = settings
                .complex_script_font_preferences
                .get(*key)
                .cloned()
                .unwrap_or_default();
            let display = if current.is_empty() {
                t!("settings.editor.complex_script_default").to_string()
            } else {
                current.clone()
            };

            layout::row(ui, index, |ui| {
                // Fixed label column: labels here run from "Thai" to
                // "Southeast Asian", so starting each control after its own
                // label made the column of dropdowns a staircase.
                layout::label_column(ui, *display_name);
                egui::ComboBox::from_id_salt(format!("complex_script_{}", key))
                    .selected_text(&display)
                    .width(layout::CONTROL_WIDTH)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(layout::CONTROL_WIDTH);
                        if ui
                            .selectable_label(
                                current.is_empty(),
                                t!("settings.editor.complex_script_default"),
                            )
                            .clicked()
                        {
                            settings.complex_script_font_preferences.remove(*key);
                            changed = true;
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for font_name in fonts::list_system_fonts() {
                                    if ui
                                        .selectable_label(font_name == &current, font_name)
                                        .clicked()
                                    {
                                        settings
                                            .complex_script_font_preferences
                                            .insert((*key).to_string(), font_name.clone());
                                        changed = true;
                                    }
                                }
                            });
                    });
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // CJK Font Preference
        layout::section_heading(ui, t!("settings.editor.cjk_preference").to_string());
        layout::description(ui, t!("settings.editor.cjk_preference_hint").to_string());
        ui.add_space(4.0);

        egui::ComboBox::from_id_salt("cjk_preference_combo")
            .selected_text(
                settings
                    .cjk_font_preference
                    .selector_display_name()
                    .to_string(),
            )
            .width(layout::CONTROL_WIDTH)
            .show_ui(ui, |ui| {
                for pref in CjkFontPreference::all() {
                    let label =
                        format!("{} - {}", pref.selector_display_name(), pref.description());
                    if ui
                        .selectable_value(&mut settings.cjk_font_preference, *pref, label)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

        changed
    }

    /// Show the Editor settings section with two-column layout for toggles.
    ///
    /// Returns true if any setting was changed.
    fn show_editor_section(
        &mut self,
        ui: &mut Ui,
        settings: &mut Settings,
        workspace_root: Option<&std::path::Path>,
    ) -> bool {
        let mut changed = false;

        layout::section_heading(ui, t!("settings.editor.title").to_string());

        // Two-column grid for basic toggles.
        //
        // `striped(false)` is explicit: egui inherits `visuals().striped`, which
        // banded this grid. Banding suits a list where a row is one thing; in a
        // two-column grid it tints pairs of unrelated settings and reads as a
        // highlight on whichever lands on an odd row.
        //
        // Ten toggles fill five rows exactly. They were previously grouped
        // thematically, which left two half-empty rows and a visible gap.
        egui::Grid::new("editor_toggles_grid")
            .num_columns(2)
            .striped(false)
            .spacing([24.0, 8.0])
            .min_col_width(180.0)
            .show(ui, |ui| {
                let mut toggle = |ui: &mut Ui, value: &mut bool, label: String, tip: String| {
                    if ui.checkbox(value, label).on_hover_text(tip).changed() {
                        changed = true;
                    }
                };

                toggle(ui, &mut settings.word_wrap,
                    t!("settings.editor.word_wrap").to_string(),
                    t!("settings.editor.word_wrap_tooltip").to_string());
                toggle(ui, &mut settings.show_line_numbers,
                    t!("settings.editor.show_line_numbers").to_string(),
                    t!("settings.editor.line_numbers_tooltip").to_string());
                ui.end_row();

                toggle(ui, &mut settings.minimap_enabled,
                    t!("settings.editor.show_minimap").to_string(),
                    t!("settings.editor.minimap_tooltip").to_string());
                toggle(ui, &mut settings.highlight_matching_pairs,
                    t!("settings.editor.highlight_brackets").to_string(),
                    t!("settings.editor.highlight_brackets_tooltip").to_string());
                ui.end_row();

                toggle(ui, &mut settings.auto_close_brackets,
                    t!("settings.editor.auto_close_brackets").to_string(),
                    t!("settings.editor.auto_close_brackets_tooltip").to_string());
                toggle(ui, &mut settings.syntax_highlighting_enabled,
                    t!("settings.editor.syntax_highlighting").to_string(),
                    t!("settings.editor.syntax_highlighting_tooltip").to_string());
                ui.end_row();

                toggle(ui, &mut settings.use_spaces,
                    t!("settings.editor.use_spaces").to_string(),
                    t!("settings.editor.use_spaces_tooltip").to_string());
                toggle(ui, &mut settings.vim_mode,
                    t!("settings.editor.vim_mode").to_string(),
                    t!("settings.editor.vim_mode_tooltip").to_string());
                ui.end_row();

                toggle(ui, &mut settings.strict_line_breaks,
                    t!("settings.editor.strict_line_breaks").to_string(),
                    t!("settings.editor.strict_line_breaks_tooltip").to_string());
                toggle(ui, &mut settings.lsp_enabled,
                    t!("settings.editor.lsp_enabled").to_string(),
                    t!("settings.editor.lsp_enabled_tooltip").to_string());
                ui.end_row();
            });

        // Markdown code execution (opt-in local subprocess — settings for future runner)
        layout::section_heading(ui, t!("settings.editor.code_execution_heading").to_string());
        layout::warning(
            ui,
            t!("settings.editor.code_execution_warning").to_string(),
            ui.visuals().warn_fg_color,
        );
        ui.add_space(8.0);

        let prev_master = settings.enable_code_execution;
        if ui
            .checkbox(
                &mut settings.enable_code_execution,
                t!("settings.editor.code_execution_enable"),
            )
            .on_hover_text(t!("settings.editor.code_execution_enable_tooltip"))
            .changed()
        {
            changed = true;
            if settings.enable_code_execution && !prev_master {
                settings.allow_shell = true;
                settings.allow_python = true;
                settings.code_execution_consent_acknowledged = true;
            }
        }

        ui.add_space(4.0);
        let exec_on = settings.enable_code_execution;

        ui.horizontal(|ui| {
            ui.add_enabled_ui(exec_on, |ui| {
                if ui
                    .checkbox(
                        &mut settings.allow_shell,
                        t!("settings.editor.code_execution_allow_shell"),
                    )
                    .on_hover_text(t!("settings.editor.code_execution_allow_shell_tooltip"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .checkbox(
                        &mut settings.allow_python,
                        t!("settings.editor.code_execution_allow_python"),
                    )
                    .on_hover_text(t!("settings.editor.code_execution_allow_python_tooltip"))
                    .changed()
                {
                    changed = true;
                }
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(exec_on, |ui| {
                ui.label(format!(
                    "{}: ",
                    t!("settings.editor.code_execution_timeout")
                ));
                ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
                if ui
                    .add(
                        egui::Slider::new(
                            &mut settings.code_execution_timeout_secs,
                            Settings::MIN_CODE_EXECUTION_TIMEOUT_SECS
                                ..=Settings::MAX_CODE_EXECUTION_TIMEOUT_SECS,
                        )
                        .suffix(format!(
                            " {}",
                            t!("settings.editor.code_execution_seconds_suffix")
                        )),
                    )
                    .on_hover_text(t!("settings.editor.code_execution_timeout_tooltip"))
                    .changed()
                {
                    changed = true;
                }
            });
        });

        ui.add_space(6.0);
        ui.add_enabled_ui(exec_on, |ui| {
            if ui
                .checkbox(
                    &mut settings.code_execution_show_inline_output,
                    t!("settings.editor.code_execution_show_inline_output"),
                )
                .on_hover_text(t!(
                    "settings.editor.code_execution_show_inline_output_tooltip"
                ))
                .changed()
            {
                changed = true;
            }
        });

        // Language server binary overrides (Editor section)
        layout::section_heading(ui, "Language servers");
        layout::description(
            ui,
            "Optional path to the server binary per detected server. Leave empty to use PATH.",
        );

        let mut server_keys: Vec<String> = workspace_root
            .map(|root| {
                crate::lsp::detect_servers_for_workspace(root)
                    .into_iter()
                    .map(|(k, _)| k)
                    .collect()
            })
            .unwrap_or_default();
        for k in settings.lsp_server_overrides.keys() {
            if !server_keys.contains(k) {
                server_keys.push(k.clone());
            }
        }
        server_keys.sort();
        server_keys.dedup();

        if workspace_root.is_none() {
            ui.label(
                RichText::new(
                    "Open a folder workspace to detect language servers for this project.",
                )
                .italics()
                .weak(),
            );
        } else if server_keys.is_empty() {
            ui.label(
                RichText::new(
                    "No language servers detected for this workspace (add code files or set overrides below).",
                )
                .italics()
                .weak(),
            );
        }

        for key in &server_keys {
            let key = key.clone();
            ui.horizontal(|ui| {
                ui.label(format!("{}:", key));
                let mut path = settings
                    .lsp_server_overrides
                    .get(&key)
                    .cloned()
                    .unwrap_or_default();
                let desired = f32::max(120.0, f32::min(ui.available_width() - 72.0, 420.0));
                let te = egui::TextEdit::singleline(&mut path)
                    .desired_width(desired)
                    .hint_text("default from PATH");
                if ui.add(te).changed() {
                    if path.trim().is_empty() {
                        settings.lsp_server_overrides.remove(&key);
                    } else {
                        settings.lsp_server_overrides.insert(key, path);
                    }
                    changed = true;
                }
            });
        }

        // Default language for syntax highlighting (only show when syntax highlighting is enabled)
        if settings.syntax_highlighting_enabled {
            layout::section_heading(ui, t!("settings.editor.default_language").to_string());
            layout::description(ui, t!("settings.editor.default_language_hint").to_string());
            ui.add_space(4.0);

            let languages = crate::markdown::syntax::get_available_languages();
            let current_display = if settings.default_syntax_language.is_empty() {
                t!("settings.editor.default_language_auto").to_string()
            } else {
                languages
                    .iter()
                    .find(|(id, _)| id == &settings.default_syntax_language)
                    .map(|(_, display)| display.clone())
                    .unwrap_or_else(|| settings.default_syntax_language.clone())
            };

            egui::ComboBox::from_id_salt("default_language_combo")
                .selected_text(&current_display)
                .width(layout::CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    ui.set_min_width(layout::CONTROL_WIDTH);
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            // Auto option (empty string = no default)
                            if ui
                                .selectable_label(
                                    settings.default_syntax_language.is_empty(),
                                    t!("settings.editor.default_language_auto"),
                                )
                                .clicked()
                            {
                                settings.default_syntax_language = String::new();
                                changed = true;
                            }

                            ui.separator();

                            // List all available languages
                            for (lang_id, display_name) in &languages {
                                if ui
                                    .selectable_label(
                                        &settings.default_syntax_language == lang_id,
                                        display_name,
                                    )
                                    .clicked()
                                {
                                    settings.default_syntax_language = lang_id.clone();
                                    changed = true;
                                }
                            }
                        });
                });

            ui.add_space(4.0);
        }

        // Minimap mode selector (only show if minimap is enabled) - full width
        if settings.minimap_enabled {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(t!("settings.editor.minimap_mode")).small());
                ui.add_space(8.0);
                for mode in MinimapMode::all() {
                    let label = match mode {
                        MinimapMode::Auto => t!("settings.editor.minimap_mode_auto").to_string(),
                        MinimapMode::Semantic => {
                            t!("settings.editor.minimap_mode_semantic").to_string()
                        }
                        MinimapMode::Pixel => t!("settings.editor.minimap_mode_pixel").to_string(),
                    };
                    let desc = match mode {
                        MinimapMode::Auto => {
                            t!("settings.editor.minimap_mode_auto_desc").to_string()
                        }
                        MinimapMode::Semantic => {
                            t!("settings.editor.minimap_mode_semantic_desc").to_string()
                        }
                        MinimapMode::Pixel => {
                            t!("settings.editor.minimap_mode_pixel_desc").to_string()
                        }
                    };
                    if ui
                        .selectable_value(&mut settings.minimap_mode, *mode, &label)
                        .on_hover_text(&desc)
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Tab size - compact horizontal layout
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.tab_size")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);

            let mut tab_size_f32 = settings.tab_size as f32;
            ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
            let tab_slider = ui.add(
                egui::Slider::new(
                    &mut tab_size_f32,
                    Settings::MIN_TAB_SIZE as f32..=Settings::MAX_TAB_SIZE as f32,
                )
                .show_value(true)
                .suffix(format!(" {}", t!("settings.editor.spaces")))
                .step_by(1.0),
            );
            if tab_slider.changed() {
                settings.tab_size = tab_size_f32 as u8;
                changed = true;
            }

            ui.add_space(8.0);
            for size in [2u8, 4, 8] {
                if ui.small_button(format!("{}", size)).clicked() {
                    settings.tab_size = size;
                    changed = true;
                }
            }
        });

        ui.add_space(8.0);

        // Maximum Line Width - compact layout
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.max_line_width")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);

            let current_display = settings.max_line_width.display_name();
            egui::ComboBox::from_id_salt("max_line_width_combo")
                .selected_text(current_display)
                .width(layout::CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for preset in MaxLineWidth::presets() {
                        let label = format!("{} - {}", preset.display_name(), preset.description());
                        if ui
                            .selectable_value(&mut settings.max_line_width, *preset, label)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                    let is_custom = settings.max_line_width.is_custom();
                    let custom_label = t!("settings.editor.custom_width");
                    if ui
                        .selectable_label(is_custom, custom_label.to_string())
                        .clicked()
                        && !is_custom
                    {
                        settings.max_line_width = MaxLineWidth::Custom(800);
                        changed = true;
                    }
                });

            // Show inline numeric input when custom is selected
            if let MaxLineWidth::Custom(px) = &mut settings.max_line_width {
                let mut px_value = *px as f32;
                let drag = ui.add(
                    egui::DragValue::new(&mut px_value)
                        .speed(10.0)
                        .range(
                            Settings::MIN_CUSTOM_LINE_WIDTH as f32
                                ..=Settings::MAX_CUSTOM_LINE_WIDTH as f32,
                        )
                        .suffix("px"),
                );
                if drag.changed() {
                    *px = px_value as u32;
                    changed = true;
                }
            }
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Code Folding section - compact two-column when enabled
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.code_folding")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            if ui
                .checkbox(
                    &mut settings.folding_enabled,
                    t!("settings.editor.enable_folding"),
                )
                .on_hover_text(t!("settings.editor.folding_tooltip"))
                .changed()
            {
                changed = true;
            }
        });

        if settings.folding_enabled {
            ui.add_space(4.0);

            // Fold options in a grid
            ui.indent("fold_options", |ui| {
                // Not banded, for the same reason as `editor_toggles_grid`:
                // this is a grid, not a list.
                egui::Grid::new("fold_options_grid")
                    .num_columns(2)
                    .striped(false)
                    .spacing([24.0, 8.0])
                    .min_col_width(180.0)
                    .show(ui, |ui| {
                        // Row 1: Show indicators | Headings
                        if ui
                            .checkbox(
                                &mut settings.folding_show_indicators,
                                t!("settings.editor.show_fold_indicators"),
                            )
                            .on_hover_text(t!("settings.editor.fold_indicators_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .checkbox(
                                &mut settings.fold_headings,
                                t!("settings.editor.fold_headings"),
                            )
                            .on_hover_text(t!("settings.editor.fold_headings_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 2: Code Blocks | Lists
                        if ui
                            .checkbox(
                                &mut settings.fold_code_blocks,
                                t!("settings.editor.fold_code_blocks"),
                            )
                            .on_hover_text(t!("settings.editor.fold_code_blocks_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .checkbox(&mut settings.fold_lists, t!("settings.editor.fold_lists"))
                            .on_hover_text(t!("settings.editor.fold_lists_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();

                        // Row 3: Indentation (single item)
                        if ui
                            .checkbox(
                                &mut settings.fold_indentation,
                                t!("settings.editor.fold_indentation"),
                            )
                            .on_hover_text(t!("settings.editor.fold_indentation_tooltip"))
                            .changed()
                        {
                            changed = true;
                        }
                        ui.end_row();
                    });
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Snippets section - compact
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.snippets")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            if ui
                .checkbox(
                    &mut settings.snippets_enabled,
                    t!("settings.editor.enable_snippets"),
                )
                .on_hover_text(t!("settings.editor.snippets_tooltip"))
                .changed()
            {
                changed = true;
            }
        });

        if settings.snippets_enabled {
            ui.add_space(4.0);
            ui.indent("snippets_info", |ui| {
                ui.label(RichText::new(t!("settings.editor.builtin_snippets")).small());
                // Two-column snippet display
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t!("settings.editor.snippet_date"))
                            .code()
                            .small(),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(t!("settings.editor.snippet_time"))
                            .code()
                            .small(),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t!("settings.editor.snippet_datetime"))
                            .code()
                            .small(),
                    );
                    ui.add_space(16.0);
                    ui.label(
                        RichText::new(t!("settings.editor.snippet_now"))
                            .code()
                            .small(),
                    );
                });
            });
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // CJK Paragraph Indentation - compact
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.paragraph_indent")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);

            use crate::config::ParagraphIndent;
            let current_display = settings.paragraph_indent.display_name();
            egui::ComboBox::from_id_salt("paragraph_indent_combo")
                .selected_text(current_display)
                .width(layout::CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for preset in ParagraphIndent::presets() {
                        let label = format!("{} - {}", preset.display_name(), preset.description());
                        if ui
                            .selectable_value(&mut settings.paragraph_indent, *preset, label)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                    let is_custom = settings.paragraph_indent.is_custom();
                    let custom_label = t!("settings.editor.paragraph_indent_custom");
                    if ui
                        .selectable_label(
                            is_custom,
                            format!(
                                "{} - {}",
                                custom_label,
                                t!("settings.editor.paragraph_indent_custom_desc")
                            ),
                        )
                        .clicked()
                        && !is_custom
                    {
                        settings.paragraph_indent = ParagraphIndent::Custom(20);
                        changed = true;
                    }
                });

            // Show inline numeric input when custom is selected
            if let ParagraphIndent::Custom(tenths) = &mut settings.paragraph_indent {
                let mut em_value = *tenths as f32 / 10.0;
                let drag = ui.add(
                    egui::DragValue::new(&mut em_value)
                        .speed(0.1)
                        .range(0.5..=5.0)
                        .suffix("em"),
                );
                if drag.changed() {
                    *tenths = (em_value * 10.0).round() as u8;
                    changed = true;
                }
            }
        });

        // Hint text for paragraph indent
        layout::description(ui, t!("settings.editor.paragraph_indent_hint").to_string());

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // Header Spacing (Markdown Rendering)
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.editor.header_spacing")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);

            let current_display = settings.header_spacing.display_name();
            egui::ComboBox::from_id_salt("header_spacing_combo")
                .selected_text(current_display)
                .width(layout::CONTROL_WIDTH)
                .show_ui(ui, |ui| {
                    for preset in HeaderSpacing::presets() {
                        let label = format!("{} - {}", preset.display_name(), preset.description());
                        if ui
                            .selectable_value(&mut settings.header_spacing, *preset, label)
                            .changed()
                        {
                            changed = true;
                        }
                    }
                });
        });

        layout::description(ui, t!("settings.editor.header_spacing_hint").to_string());

        changed
    }

    /// Show the Files settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_files_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        layout::section_heading(ui, t!("settings.files.title").to_string());

        // Session restore toggle
        if ui
            .checkbox(
                &mut settings.restore_session,
                t!("settings.general.restore_session"),
            )
            .on_hover_text(t!("settings.files.restore_session_tooltip"))
            .changed()
        {
            changed = true;
        }

        if ui
            .checkbox(
                &mut settings.quick_note_workflow,
                t!("settings.files.quick_note_workflow"),
            )
            .on_hover_text(t!("settings.files.quick_note_workflow_tooltip"))
            .changed()
        {
            changed = true;
        }
        layout::description(ui, t!("settings.files.quick_note_workflow_hint").to_string());

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Auto-save toggle (default for new documents)
        if ui
            .checkbox(
                &mut settings.auto_save_enabled_default,
                t!("settings.files.enable_auto_save"),
            )
            .on_hover_text(t!("settings.files.auto_save_tooltip"))
            .changed()
        {
            changed = true;
        }

        ui.add_space(4.0);

        // Auto-save delay
        ui.horizontal(|ui| {
            ui.label(t!("settings.files.auto_save_delay"));
            ui.add_space(8.0);
            let secs = settings.auto_save_delay_ms / 1000;
            ui.label(t!("settings.files.seconds", count = secs));
        });
        ui.add_space(4.0);

        // Convert ms to seconds for slider display
        let mut delay_secs = (settings.auto_save_delay_ms / 1000) as f32;
        ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
        let delay_slider = ui.add(
            egui::Slider::new(&mut delay_secs, 5.0..=300.0)
                .show_value(false)
                .step_by(5.0),
        );
        if delay_slider.changed() {
            settings.auto_save_delay_ms = (delay_secs as u32) * 1000;
            changed = true;
        }

        // Delay presets
        ui.horizontal(|ui| {
            for (label, ms) in [("15s", 15000), ("30s", 30000), ("1m", 60000)] {
                if ui.small_button(label).clicked() {
                    settings.auto_save_delay_ms = ms;
                    changed = true;
                }
            }
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // Recent files count
        ui.horizontal(|ui| {
            ui.label(RichText::new(t!("settings.files.recent_files")).font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)));
            ui.add_space(8.0);
            ui.label(t!(
                "settings.files.remember_files",
                count = settings.max_recent_files
            ));
        });
        ui.add_space(4.0);

        let mut recent_count_f32 = settings.max_recent_files as f32;
        ui.spacing_mut().slider_width = layout::SLIDER_WIDTH;
        let recent_slider = ui.add(
            egui::Slider::new(&mut recent_count_f32, 0.0..=20.0)
                .show_value(false)
                .step_by(1.0),
        );
        if recent_slider.changed() {
            settings.max_recent_files = recent_count_f32 as usize;
            changed = true;
        }

        ui.add_space(8.0);

        // Clear recent files button
        ui.horizontal(|ui| {
            if ui
                .button(t!("settings.files.clear_recent"))
                .on_hover_text(t!("settings.files.clear_recent_tooltip"))
                .clicked()
            {
                settings.recent_files.clear();
                changed = true;
            }

            if !settings.recent_files.is_empty() {
                ui.label(
                    RichText::new(t!(
                        "settings.files.files_count",
                        count = settings.recent_files.len()
                    ))
                    .small()
                    .weak(),
                );
            }
        });

        changed
    }

    /// Show the Keyboard shortcuts settings section.
    ///
    /// Returns true if any setting was changed.
    fn show_keyboard_section(&mut self, ui: &mut Ui, settings: &mut Settings) -> bool {
        let mut changed = false;

        layout::section_heading(ui, t!("settings.keyboard.title").to_string());

        // Search/filter box
        ui.horizontal(|ui| {
            ui.label(phosphor_rich_text(MAGNIFYING_GLASS, 14.0));
            ui.add(
                egui::TextEdit::singleline(&mut self.keyboard_filter)
                    .hint_text(t!("settings.keyboard.search_hint"))
                    .desired_width(200.0),
            );
            if !self.keyboard_filter.is_empty() {
                if ui.small_button(phosphor_rich_text(X, 12.0)).clicked() {
                    self.keyboard_filter.clear();
                }
            }
        });

        ui.add_space(4.0);

        // Reset all button
        ui.horizontal(|ui| {
            if ui
                .button(t!("settings.keyboard.reset_all").to_string())
                .on_hover_text(t!("settings.keyboard.reset_all_tooltip"))
                .clicked()
            {
                settings.keyboard_shortcuts.reset_all();
                self.conflict_warning = None;
                changed = true;
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // Show conflict warning if any
        if let Some((cmd, msg)) = &self.conflict_warning {
            let warn_color = ui.visuals().warn_fg_color;
            ui.horizontal(|ui| {
                ui.label(phosphor_rich_text(WARNING, 14.0).color(warn_color));
                ui.label(
                    RichText::new(format!("{}: {}", shortcut_command_name(cmd), msg))
                        .color(warn_color),
                );
            });
            ui.add_space(4.0);
        }

        // Key capture modal - clone state for use in closure
        let mut cancel_capture = false;
        let mut apply_capture: Option<(ShortcutCommand, KeyBinding)> = None;

        if let Some(capture) = &self.key_capture {
            let cmd_name = shortcut_command_name(&capture.command);
            let current_mods = capture.modifiers.display_string();
            let current_key = capture.key.map(|k| k.display_string()).unwrap_or("");
            let has_key = capture.key.is_some();
            let capture_cmd = capture.command;
            let capture_mods = capture.modifiers;
            let capture_key = capture.key;

            let display = if current_mods.is_empty() && current_key.is_empty() {
                t!("settings.keyboard.waiting").to_string()
            } else if current_key.is_empty() {
                format!("{}+...", current_mods)
            } else if current_mods.is_empty() {
                current_key.to_string()
            } else {
                format!("{}+{}", current_mods, current_key)
            };

            ui.group(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} \"{}\"...",
                        t!("settings.keyboard.press_key"),
                        cmd_name
                    ))
                    .font(crate::fonts::chrome_bold_font(crate::theme::typescale::chrome::BODY)),
                );
                ui.add_space(4.0);

                ui.label(RichText::new(&display).monospace().size(chrome::BODY));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button(t!("settings.keyboard.cancel")).clicked() {
                        cancel_capture = true;
                    }
                    if has_key {
                        if ui.button(t!("settings.keyboard.apply")).clicked() {
                            if let Some(key) = capture_key {
                                apply_capture =
                                    Some((capture_cmd, KeyBinding::new(capture_mods, key)));
                            }
                        }
                    }
                });
            });
            ui.add_space(8.0);
        }

        // Handle deferred capture actions
        if cancel_capture {
            self.key_capture = None;
        }
        if let Some((cmd, binding)) = apply_capture {
            // Check for conflicts
            if let Some(conflict_cmd) = settings
                .keyboard_shortcuts
                .find_conflict(&binding, Some(cmd))
            {
                self.conflict_warning = Some((
                    cmd,
                    format!(
                        "{} \"{}\"",
                        t!("settings.keyboard.conflict_with"),
                        shortcut_command_name(&conflict_cmd)
                    ),
                ));
            } else {
                settings.keyboard_shortcuts.set(cmd, binding);
                self.conflict_warning = None;
                changed = true;
            }
            self.key_capture = None;
        }

        // Capture keyboard input when in capture mode
        let mut escape_pressed = false;
        let mut new_modifiers: Option<KeyModifiers> = None;
        let mut new_key: Option<KeyCode> = None;

        // Check if we already have a key captured (to latch modifiers)
        let key_already_captured = self
            .key_capture
            .as_ref()
            .map(|c| c.key.is_some())
            .unwrap_or(false);

        if self.key_capture.is_some() {
            ui.input(|i| {
                // Only update modifiers if no key captured yet (latch them once key is pressed)
                if !key_already_captured {
                    new_modifiers = Some(KeyModifiers::from_egui(&i.modifiers));
                }

                // Check for key press
                for event in &i.events {
                    if let egui::Event::Key {
                        key, pressed: true, ..
                    } = event
                    {
                        // Skip modifier-only keys
                        if matches!(key, egui::Key::Escape) {
                            escape_pressed = true;
                            return;
                        }
                        if let Some(key_code) = KeyCode::from_egui(*key) {
                            new_key = Some(key_code);
                            // Capture modifiers at the moment the key is pressed
                            new_modifiers = Some(KeyModifiers::from_egui(&i.modifiers));
                        }
                    }
                }
            });
        }

        // Apply captured input
        if escape_pressed {
            self.key_capture = None;
        } else if let Some(capture) = &mut self.key_capture {
            // Only update modifiers if no key captured yet, or if capturing new key with its modifiers
            if capture.key.is_none() {
                if let Some(mods) = new_modifiers {
                    capture.modifiers = mods;
                }
            }
            if let Some(key) = new_key {
                capture.key = Some(key);
                // Also latch the modifiers that came with the key press
                if let Some(mods) = new_modifiers {
                    capture.modifiers = mods;
                }
            }
        }

        // Scrollable area for shortcuts list
        // Uses available height - the outer container (modal or inline tab) handles overflow
        let filter_lower = self.keyboard_filter.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (category, commands) in KeyboardShortcuts::commands_by_category() {
                // Filter commands by search term
                let filtered_commands: Vec<_> = commands
                    .iter()
                    .filter(|cmd| {
                        if filter_lower.is_empty() {
                            return true;
                        }
                        shortcut_command_name(cmd)
                            .to_lowercase()
                            .contains(&filter_lower)
                            || category.to_lowercase().contains(&filter_lower)
                    })
                    .collect();

                if filtered_commands.is_empty() {
                    continue;
                }

                // Category header — row banding resets here so it reads within
                // this group rather than running across the whole list.
                layout::section_heading(ui, category.to_string());

                for (row_index, &cmd) in filtered_commands.iter().enumerate() {
                    let binding = settings.keyboard_shortcuts.get(*cmd);
                    let is_custom = settings.keyboard_shortcuts.is_custom(*cmd);
                    let is_capturing = self
                        .key_capture
                        .as_ref()
                        .map(|c| c.command == *cmd)
                        .unwrap_or(false);

                    layout::row(ui, row_index, |ui| {
                        // Command name
                        let cmd_name = shortcut_command_name(cmd);
                        let name_text = if is_custom {
                            RichText::new(&cmd_name).italics()
                        } else {
                            RichText::new(&cmd_name)
                        };
                        ui.label(name_text);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Reset button (only if custom)
                            if is_custom {
                                if ui
                                    .small_button("↺")
                                    .on_hover_text(t!("settings.keyboard.reset_default"))
                                    .clicked()
                                {
                                    settings.keyboard_shortcuts.reset(*cmd);
                                    changed = true;
                                }
                            }

                            // Binding button
                            let btn_text = if is_capturing {
                                "...".to_string()
                            } else {
                                binding.display_string()
                            };
                            let btn = ui.add(
                                egui::Button::new(RichText::new(&btn_text).monospace())
                                    .min_size(egui::vec2(100.0, 0.0)),
                            );
                            if btn.clicked() && self.key_capture.is_none() {
                                self.key_capture = Some(KeyCaptureState {
                                    command: *cmd,
                                    modifiers: KeyModifiers::none(),
                                    key: None,
                                });
                                self.conflict_warning = None;
                            }
                            if btn.hovered() && !is_capturing {
                                btn.on_hover_text(t!("settings.keyboard.click_to_change"));
                            }
                        });
                    });
                }
            }
        });

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_panel_new() {
        let panel = SettingsPanel::new();
        assert_eq!(panel.active_section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_panel_default() {
        let panel = SettingsPanel::default();
        assert_eq!(panel.active_section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_section_label() {
        assert_eq!(SettingsSection::Appearance.label(), "Appearance");
        assert_eq!(SettingsSection::Editor.label(), "Editor");
        assert_eq!(SettingsSection::Files.label(), "Files");
        assert_eq!(SettingsSection::Terminal.label(), "Terminal");
        assert_eq!(SettingsSection::About.label(), "About");
    }

    #[test]
    fn test_settings_section_icon() {
        assert_eq!(
            SettingsSection::Appearance.icon(),
            crate::ui::phosphor_icons::PALETTE
        );
        assert_eq!(
            SettingsSection::Editor.icon(),
            crate::ui::phosphor_icons::NOTE_PENCIL
        );
        assert_eq!(
            SettingsSection::Files.icon(),
            crate::ui::phosphor_icons::FILES
        );
        assert_eq!(
            SettingsSection::Terminal.icon(),
            crate::ui::phosphor_icons::TERMINAL_WINDOW
        );
        assert_eq!(
            SettingsSection::About.icon(),
            crate::ui::phosphor_icons::INFO
        );
    }

    #[test]
    fn test_settings_section_default() {
        let section = SettingsSection::default();
        assert_eq!(section, SettingsSection::Appearance);
    }

    #[test]
    fn test_settings_panel_output_default() {
        let output = SettingsPanelOutput::default();
        assert!(!output.changed);
        assert!(!output.reset_requested);
    }
}
