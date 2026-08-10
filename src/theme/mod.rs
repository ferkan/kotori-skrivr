//! Theme System for Ferrite
//!
//! This module provides a comprehensive theming system that defines colors,
//! fonts, and spacing for consistent UI styling across the application.

// Allow dead code - this module has comprehensive theme utilities for future use
#![allow(dead_code)]

//! # Architecture
//!
//! The theme system is built around the `ThemeColors` struct which contains
//! all color definitions needed for the UI. The existing `Theme` enum in
//! `config::settings` (Light/Dark/System) is used to select which palette
//! to use at runtime.
//!
//! # Usage
//!
//! ```ignore
//! use crate::theme::{ThemeColors, ThemeSpacing, ThemeFonts};
//! use crate::config::Theme;
//!
//! // Get colors for the current theme
//! let colors = ThemeColors::from_theme(Theme::Dark, &ctx.global_style().visuals);
//!
//! // Use in egui
//! ui.label(RichText::new("Hello").color(colors.text.primary));
//!
//! // Apply theme to egui context
//! let visuals = colors.to_visuals();
//! ctx.set_visuals(visuals);
//! ```
//!
//! # Theme Files
//!
//! - `light.rs` - Light theme configuration and egui Visuals
//! - `dark.rs` - Dark theme configuration and egui Visuals
//! - `colors.rs` - Color constants and utilities
//!
//! # Color Categories
//!
//! - **Base colors**: Background, foreground, borders
//! - **Text colors**: Primary, secondary, muted, link, code
//! - **Editor colors**: Headings, blockquotes, code blocks, lists
//! - **Syntax colors**: Keywords, strings, comments, etc.
//! - **UI colors**: Accent, success, warning, error

pub mod accent;
pub mod typescale;
#[cfg(test)]
mod contrast_tests;
pub mod dark;
pub mod light;
pub mod manager;

pub use manager::ThemeManager;

use eframe::egui::Color32;

// ─────────────────────────────────────────────────────────────────────────────
// Theme Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Comprehensive theme colors for the entire application.
///
/// This struct consolidates all color definitions needed for consistent
/// UI theming, replacing the fragmented `EditorColors` and `WidgetColors`
/// with a unified system.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemeColors {
    /// Base UI colors (backgrounds, borders)
    pub base: BaseColors,
    /// Text colors for various contexts
    pub text: TextColors,
    /// Colors for the markdown editor
    pub editor: EditorThemeColors,
    /// Syntax highlighting colors for code blocks
    pub syntax: SyntaxColors,
    /// UI feedback colors (success, warning, error)
    pub ui: UiColors,
}

impl ThemeColors {
    /// Apply the user-selected accent (checkbox, UI accent, selection).
    /// Markdown / UI hyperlinks intentionally stay [`accent::standard_link_color`].
    ///
    /// Headings deliberately do NOT take the accent: hierarchy is carried by
    /// size and weight (`theme::typescale`), and a fourth signal on top is
    /// redundant — the accent now means "you can act here" (a checkbox is an
    /// interactive control; a heading is not).
    pub fn apply_user_accent(&mut self, accent: Color32) {
        let dark = self.is_dark();
        // Checkboxes are document content sitting on the page background,
        // not chrome sitting on a panel fill. The raw accent is tuned for
        // the latter: the default light blue measures 7.6:1 on the dark
        // panel but 2.2:1 on a white page, below even the 3:1 large-text
        // floor. `readable_on` keeps the user's hue and only darkens/lightens
        // far enough to clear AA, so a well-chosen accent passes through
        // unchanged. This also governs HTML export (`export/html.rs:85`).
        let on_page = accent::readable_on(accent, self.base.background, accent::MIN_TEXT_CONTRAST);
        self.editor.checkbox = on_page;

        // Chrome painted *in* the accent has to be discernible against the
        // ground it sits on, and one accent serves both themes. The Kotori
        // green default measures 8.69:1 on the warm page but 1.88:1 on the
        // warm charcoal — invisible as a heading colour or a selected fill.
        // Lift it just far enough to clear the non-text floor, keeping the
        // hue. An accent that already clears passes through untouched, so
        // this changes nothing for well-chosen values; it is a floor, not a
        // restyling. User accents are unconstrained in the welcome picker and
        // hit exactly the same problem.
        let on_ground = accent::readable_on(accent, self.base.background, accent::MIN_UI_CONTRAST);
        self.ui.accent = on_ground;
        self.ui.accent_hover = accent::accent_hover(on_ground, dark);
        self.base.selected = accent::selection_fill(on_ground, dark);
    }

    /// Create theme colors for the given theme variant with a Ferrite accent.
    ///
    /// This is the primary way to get themed colors. It automatically
    /// selects the appropriate palette based on the theme setting.
    pub fn from_theme(
        theme: crate::config::Theme,
        visuals: &eframe::egui::Visuals,
        accent: Color32,
    ) -> Self {
        let mut palette = match theme {
            crate::config::Theme::Dark => Self::dark(),
            crate::config::Theme::Light => Self::light(),
            crate::config::Theme::System => {
                if visuals.dark_mode {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        };
        palette.apply_user_accent(accent);
        palette
    }

    /// Get the light theme colors.
    pub fn light() -> Self {
        Self {
            base: BaseColors::light(),
            text: TextColors::light(),
            editor: EditorThemeColors::light(),
            syntax: SyntaxColors::light(),
            ui: UiColors::light(),
        }
    }

    /// Get the dark theme colors.
    pub fn dark() -> Self {
        Self {
            base: BaseColors::dark(),
            text: TextColors::dark(),
            editor: EditorThemeColors::dark(),
            syntax: SyntaxColors::dark(),
            ui: UiColors::dark(),
        }
    }

    /// Check if this is a dark theme (useful for conditional styling).
    pub fn is_dark(&self) -> bool {
        // Dark themes have darker backgrounds
        self.base.background.r() < 128
    }

    /// Convert theme colors to egui Visuals for UI styling.
    ///
    /// This is the primary method to apply the theme to egui. It creates
    /// a complete `Visuals` struct configured with the theme's colors.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::theme::ThemeColors;
    ///
    /// let colors = ThemeColors::dark();
    /// let visuals = colors.to_visuals();
    /// ctx.set_visuals(visuals);
    /// ```
    pub fn to_visuals(&self) -> eframe::egui::Visuals {
        if self.is_dark() {
            dark::visuals_from_palette(self)
        } else {
            light::visuals_from_palette(self)
        }
    }

    /// Create visuals for the given theme variant.
    ///
    /// Convenience method that combines `from_theme` and `to_visuals`.
    pub fn visuals_for_theme(
        theme: crate::config::Theme,
        system_visuals: &eframe::egui::Visuals,
        accent: Color32,
    ) -> eframe::egui::Visuals {
        Self::from_theme(theme, system_visuals, accent).to_visuals()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Base Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Base UI colors for backgrounds and borders.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseColors {
    /// Primary background color
    pub background: Color32,
    /// Secondary/elevated background (panels, cards)
    pub background_secondary: Color32,
    /// Tertiary background (inputs, code blocks)
    pub background_tertiary: Color32,
    /// Primary border color
    pub border: Color32,
    /// Subtle border color (dividers)
    pub border_subtle: Color32,
    /// Hover state background
    pub hover: Color32,
    /// Selected/active state background
    pub selected: Color32,
}

impl BaseColors {
    /// Light theme base colors — warm paper palette.
    ///
    /// Contrast ratios against the theme's own background (251,249,245):
    /// - border: 3.08:1 (meets WCAG AA for UI components)
    /// - border_subtle: 1.43:1 (decorative divider only, not a component boundary)
    /// - hover/selected: sufficient visual distinction
    pub fn light() -> Self {
        Self {
            background: Color32::from_rgb(251, 249, 245),
            background_secondary: Color32::from_rgb(244, 241, 234),
            background_tertiary: Color32::from_rgb(237, 233, 224),
            border: Color32::from_rgb(150, 142, 128),
            border_subtle: Color32::from_rgb(216, 210, 198),
            hover: Color32::from_rgb(240, 236, 228),
            // Saturated enough to remain visible at the ~40% alpha the
            // FerriteEditor selection overlay applies on top of text (#121).
            // At full alpha this also works as a light-theme widget-selected
            // color without being harsh.
            selected: Color32::from_rgb(250, 206, 184),
        }
    }

    /// Dark theme base colors — warm charcoal palette.
    ///
    /// Contrast ratios against the theme's own background (28,27,25):
    /// - border: 3.57:1 (meets WCAG AA for UI components)
    /// - border_subtle: 1.37:1 (decorative divider only, not a component boundary)
    pub fn dark() -> Self {
        Self {
            background: Color32::from_rgb(28, 27, 25),
            background_secondary: Color32::from_rgb(35, 33, 32),
            background_tertiary: Color32::from_rgb(42, 39, 36),
            border: Color32::from_rgb(120, 113, 104),
            border_subtle: Color32::from_rgb(55, 51, 47),
            hover: Color32::from_rgb(44, 41, 38),
            selected: Color32::from_rgb(74, 55, 45),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Text Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Text colors for various contexts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextColors {
    /// Primary text color (main content)
    pub primary: Color32,
    /// Secondary text color (descriptions, labels)
    pub secondary: Color32,
    /// Muted text color (hints, placeholders)
    pub muted: Color32,
    /// Disabled text color
    pub disabled: Color32,
    /// Link text color
    pub link: Color32,
    /// Code text color (inline code)
    pub code: Color32,
}

impl TextColors {
    /// Light theme text colors — warm palette.
    ///
    /// Contrast ratios against the theme's background (251,249,245):
    /// - primary: 15.46:1 (exceeds WCAG AAA)
    /// - secondary: 7.59:1 (exceeds WCAG AAA)
    /// - muted: 5.24:1 (exceeds WCAG AA)
    /// - disabled: 3.47:1 (disabled exempt from WCAG, kept above 3:1)
    /// - link: 6.56:1 (exceeds WCAG AA)
    /// - code: 6.90:1 (exceeds WCAG AA)
    pub fn light() -> Self {
        Self {
            primary: Color32::from_rgb(34, 32, 28),
            secondary: Color32::from_rgb(85, 80, 74),
            muted: Color32::from_rgb(110, 104, 96),
            disabled: Color32::from_rgb(140, 133, 122),
            link: Color32::from_rgb(0, 90, 170),
            code: Color32::from_rgb(92, 86, 78),
        }
    }

    /// Dark theme text colors — warm palette.
    ///
    /// Contrast ratios against the theme's background (28,27,25):
    /// - primary: 13.58:1 (exceeds WCAG AAA)
    /// - secondary: 7.93:1 (exceeds WCAG AAA)
    /// - muted: 5.04:1 (exceeds WCAG AA)
    /// - disabled: 3.36:1 (disabled exempt from WCAG, kept above 3:1)
    /// - link: 7.80:1 (exceeds WCAG AA)
    /// - code: 10.08:1 (exceeds WCAG AAA)
    pub fn dark() -> Self {
        Self {
            primary: Color32::from_rgb(232, 228, 221),
            secondary: Color32::from_rgb(182, 175, 166),
            muted: Color32::from_rgb(145, 138, 128),
            disabled: Color32::from_rgb(115, 109, 100),
            link: Color32::from_rgb(100, 180, 255),
            code: Color32::from_rgb(206, 199, 160),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Editor Theme Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors specific to the markdown editor.
///
/// These colors are used for rendering markdown elements in both
/// raw and WYSIWYG (rendered) editing modes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorThemeColors {
    /// Heading text color (H1-H6). Equal to `TextColors::primary` — headings
    /// are not accent-tinted; hierarchy comes from size and weight.
    pub heading: Color32,
    /// Block quote border color
    pub blockquote_border: Color32,
    /// Block quote text color
    pub blockquote_text: Color32,
    /// Code block background color
    pub code_block_bg: Color32,
    /// Code block border color
    pub code_block_border: Color32,
    /// Horizontal rule color
    pub horizontal_rule: Color32,
    /// List marker color (bullets, numbers)
    pub list_marker: Color32,
    /// Task checkbox color
    pub checkbox: Color32,
    /// Table border color
    pub table_border: Color32,
    /// Table header background
    pub table_header_bg: Color32,
}

impl EditorThemeColors {
    /// Light theme editor colors — warm palette.
    ///
    /// `checkbox` is a placeholder overwritten by
    /// [`ThemeColors::apply_user_accent`] at runtime, so it's left unchanged
    /// rather than retuned here. `heading` is not: it equals
    /// `TextColors::light().primary` and stays that way — headings are no
    /// longer accent-tinted.
    ///
    /// Contrast ratios against the theme's background (251,249,245), unless
    /// noted otherwise:
    /// - heading: 15.46:1 (identical to `text.primary`, exceeds WCAG AAA)
    /// - blockquote_border / horizontal_rule: 3.08:1 (identical to `base.border`,
    ///   meets the UI component floor)
    /// - blockquote_text / list_marker: 6.68:1 (exceeds WCAG AA; identical values,
    ///   as before)
    /// - table_border: 3.08:1 (identical to `base.border`)
    /// - code_block_bg / table_header_bg: not text, no contrast floor — matched
    ///   to `base.background_tertiary` / `base.background_secondary`
    pub fn light() -> Self {
        Self {
            heading: Color32::from_rgb(34, 32, 28),
            blockquote_border: Color32::from_rgb(150, 142, 128),
            blockquote_text: Color32::from_rgb(94, 88, 80),
            code_block_bg: Color32::from_rgb(237, 233, 224),
            code_block_border: Color32::from_rgb(178, 171, 157),
            horizontal_rule: Color32::from_rgb(150, 142, 128),
            list_marker: Color32::from_rgb(94, 88, 80),
            checkbox: Color32::from_rgb(0, 90, 165),
            table_border: Color32::from_rgb(150, 142, 128),
            table_header_bg: Color32::from_rgb(244, 241, 234),
        }
    }

    /// Dark theme editor colors — warm palette.
    ///
    /// `checkbox` is a placeholder overwritten by
    /// [`ThemeColors::apply_user_accent`] at runtime, so it's left unchanged
    /// rather than retuned here. `heading` is not: it equals
    /// `TextColors::dark().primary` and stays that way — headings are no
    /// longer accent-tinted.
    ///
    /// Contrast ratios against the theme's background (28,27,25), unless
    /// noted otherwise:
    /// - heading: 13.58:1 (identical to `text.primary`, exceeds WCAG AAA)
    /// - blockquote_border / horizontal_rule: 3.57:1 (identical to `base.border`,
    ///   meets the UI component floor)
    /// - blockquote_text / list_marker: 7.57:1 (exceeds WCAG AA; identical values,
    ///   as before)
    /// - table_border: 3.57:1 (identical to `base.border`)
    /// - code_block_bg / table_header_bg: not text, no contrast floor — dark
    ///   panel tones between `base.background_secondary` and `base.background_tertiary`
    pub fn dark() -> Self {
        Self {
            heading: Color32::from_rgb(232, 228, 221),
            blockquote_border: Color32::from_rgb(120, 113, 104),
            blockquote_text: Color32::from_rgb(178, 171, 161),
            code_block_bg: Color32::from_rgb(38, 36, 33),
            code_block_border: Color32::from_rgb(68, 63, 57),
            horizontal_rule: Color32::from_rgb(120, 113, 104),
            list_marker: Color32::from_rgb(178, 171, 161),
            checkbox: Color32::from_rgb(100, 180, 255),
            table_border: Color32::from_rgb(120, 113, 104),
            table_header_bg: Color32::from_rgb(48, 45, 41),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Syntax Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for syntax highlighting in code blocks.
///
/// These colors are used when syntax highlighting is not available
/// or as fallback colors. The full syntax highlighting uses syntect
/// themes which have their own color definitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntaxColors {
    /// Keyword color (if, else, fn, let, etc.)
    pub keyword: Color32,
    /// String literal color
    pub string: Color32,
    /// Number literal color
    pub number: Color32,
    /// Comment color
    pub comment: Color32,
    /// Function name color
    pub function: Color32,
    /// Type/class name color
    pub type_name: Color32,
    /// Variable name color
    pub variable: Color32,
    /// Operator color (+, -, =, etc.)
    pub operator: Color32,
    /// Punctuation color (brackets, semicolons)
    pub punctuation: Color32,
}

impl SyntaxColors {
    /// Light theme syntax colors.
    pub fn light() -> Self {
        Self {
            keyword: Color32::from_rgb(175, 0, 175),       // Purple
            string: Color32::from_rgb(0, 128, 0),          // Green
            number: Color32::from_rgb(0, 128, 128),        // Teal
            comment: Color32::from_rgb(128, 128, 128),     // Gray
            function: Color32::from_rgb(0, 0, 175),        // Blue
            type_name: Color32::from_rgb(0, 100, 150),     // Dark cyan
            variable: Color32::from_rgb(50, 50, 50),       // Dark gray
            operator: Color32::from_rgb(80, 80, 80),       // Gray
            punctuation: Color32::from_rgb(100, 100, 100), // Medium gray
        }
    }

    /// Dark theme syntax colors.
    pub fn dark() -> Self {
        Self {
            keyword: Color32::from_rgb(198, 120, 221),   // Light purple
            string: Color32::from_rgb(152, 195, 121),    // Light green
            number: Color32::from_rgb(209, 154, 102),    // Orange
            comment: Color32::from_rgb(92, 99, 112),     // Gray
            function: Color32::from_rgb(97, 175, 239),   // Light blue
            type_name: Color32::from_rgb(229, 192, 123), // Yellow
            variable: Color32::from_rgb(224, 108, 117),  // Red/pink
            operator: Color32::from_rgb(171, 178, 191),  // Light gray
            punctuation: Color32::from_rgb(150, 150, 150), // Gray
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Colors for UI feedback and interactive elements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiColors {
    /// Primary accent color (buttons, active elements)
    pub accent: Color32,
    /// Accent color for hover state
    pub accent_hover: Color32,
    /// Success color (confirmations, positive actions)
    pub success: Color32,
    /// Warning color (cautions, alerts)
    pub warning: Color32,
    /// Error color (errors, destructive actions)
    pub error: Color32,
    /// Info color (informational messages)
    pub info: Color32,
    /// Background color for matching bracket highlight
    pub matching_bracket_bg: Color32,
    /// Border color for matching bracket highlight
    pub matching_bracket_border: Color32,
}

impl UiColors {
    /// Light theme UI colors — warm palette.
    ///
    /// accent (168,71,42) vs background (251,249,245): 5.55:1, exceeds WCAG AA.
    pub fn light() -> Self {
        Self {
            accent: Color32::from_rgb(168, 71, 42),
            // Derived with `accent::accent_hover` from the accent above, so the
            // static default matches what `apply_user_accent` computes at
            // runtime. These were left as the previous blue accent's hover,
            // which made hover flip hue away from the terracotta accent.
            accent_hover: Color32::from_rgb(143, 60, 36),
            success: Color32::from_rgb(40, 167, 69),
            // 4.50:1 on the light page. The previous (255,193,7) measured
            // 1.55:1 — an amber picked for the dark theme and never re-checked
            // against the warm ground, which made the code-execution security
            // notice effectively invisible. Hue preserved, luminance corrected.
            warning: Color32::from_rgb(145, 110, 4),
            error: Color32::from_rgb(220, 53, 69),
            info: Color32::from_rgb(23, 162, 184),
            // Subtle gold/yellow tint for bracket matching - visible but not overpowering
            matching_bracket_bg: Color32::from_rgba_unmultiplied(255, 220, 100, 80),
            matching_bracket_border: Color32::from_rgb(200, 170, 50),
        }
    }

    /// Dark theme UI colors — warm palette.
    ///
    /// accent (232,150,110) vs background (28,27,25): 7.39:1, exceeds WCAG AA.
    pub fn dark() -> Self {
        Self {
            accent: Color32::from_rgb(232, 150, 110),
            // Derived with `accent::accent_hover` from the accent above — see
            // the note in `light()`.
            accent_hover: Color32::from_rgb(235, 163, 127),
            success: Color32::from_rgb(75, 210, 100),
            warning: Color32::from_rgb(255, 210, 50),
            error: Color32::from_rgb(255, 100, 100),
            info: Color32::from_rgb(80, 200, 220),
            // Subtle cyan/blue tint for bracket matching - visible on dark backgrounds
            matching_bracket_bg: Color32::from_rgba_unmultiplied(80, 180, 220, 60),
            matching_bracket_border: Color32::from_rgb(100, 180, 220),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Theme Spacing
// ─────────────────────────────────────────────────────────────────────────────

/// Spacing values for consistent layout.
///
/// These values define the standard spacing used throughout the UI
/// to maintain visual consistency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeSpacing {
    /// Extra small spacing (2px)
    pub xs: f32,
    /// Small spacing (4px)
    pub sm: f32,
    /// Medium spacing (8px)
    pub md: f32,
    /// Large spacing (16px)
    pub lg: f32,
    /// Extra large spacing (24px)
    pub xl: f32,
    /// Double extra large spacing (32px)
    pub xxl: f32,
}

impl Default for ThemeSpacing {
    fn default() -> Self {
        Self {
            xs: 2.0,
            sm: 4.0,
            md: 8.0,
            lg: 16.0,
            xl: 24.0,
            xxl: 32.0,
        }
    }
}

impl ThemeSpacing {
    /// Create the default spacing values.
    pub fn new() -> Self {
        Self::default()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::accent;

    #[test]
    fn test_theme_colors_light() {
        let colors = ThemeColors::light();

        // Light theme should have light background
        assert!(colors.base.background.r() > 200);
        assert!(!colors.is_dark());
    }

    #[test]
    fn test_theme_colors_dark() {
        let colors = ThemeColors::dark();

        // Dark theme should have dark background
        assert!(colors.base.background.r() < 50);
        assert!(colors.is_dark());
    }

    #[test]
    fn test_theme_colors_from_theme() {
        let dark_colors = ThemeColors::from_theme(
            crate::config::Theme::Dark,
            &eframe::egui::Visuals::dark(),
            accent::default_accent(),
        );
        assert!(dark_colors.is_dark());

        let light_colors = ThemeColors::from_theme(
            crate::config::Theme::Light,
            &eframe::egui::Visuals::light(),
            accent::default_accent(),
        );
        assert!(!light_colors.is_dark());
    }

    #[test]
    fn test_base_colors_light() {
        let colors = BaseColors::light();
        assert!(colors.background.r() > 200);
        assert!(colors.background_secondary.r() > 200);
    }

    #[test]
    fn test_base_colors_dark() {
        let colors = BaseColors::dark();
        assert!(colors.background.r() < 50);
        assert!(colors.background_secondary.r() < 50);
    }

    #[test]
    fn test_text_colors_contrast() {
        // Light theme: dark text on light background
        let light = TextColors::light();
        assert!(light.primary.r() < 50);

        // Dark theme: light text on dark background
        let dark = TextColors::dark();
        assert!(dark.primary.r() > 200);
    }

    /// Changed from `assert_ne!` to `assert_eq!`: headings used to carry the
    /// accent as a fourth hierarchy signal on top of size and weight; they no
    /// longer do; `editor.heading` is now defined as equal to `text.primary`.
    #[test]
    fn test_editor_colors_heading_matches_primary_text() {
        let light = EditorThemeColors::light();
        let dark = EditorThemeColors::dark();

        assert_eq!(light.heading, TextColors::light().primary);
        assert_eq!(dark.heading, TextColors::dark().primary);
    }

    #[test]
    fn test_syntax_colors_variety() {
        let light = SyntaxColors::light();

        // All syntax colors should be distinct for readability
        assert_ne!(light.keyword, light.string);
        assert_ne!(light.string, light.comment);
        assert_ne!(light.function, light.type_name);
    }

    #[test]
    fn test_ui_colors_feedback() {
        let colors = UiColors::light();

        // Success should be greenish
        assert!(colors.success.g() > colors.success.r());

        // Error should be reddish
        assert!(colors.error.r() > colors.error.g());

        // Warning should be an amber HUE — red strongest, green close behind,
        // blue far back. Deliberately not a brightness assertion: the light
        // theme's warning has to be dark enough to read on a near-white page
        // (it measured 1.55:1 as a bright amber), so `r > 200` encoded a
        // luminance the light palette cannot have.
        assert!(colors.warning.r() > colors.warning.g());
        assert!(colors.warning.g() > colors.warning.b());
        assert!(
            colors.warning.b() < colors.warning.r() / 2,
            "warning must read as amber, not brown or orange-red"
        );
    }

    #[test]
    fn test_spacing_default() {
        let spacing = ThemeSpacing::default();

        assert_eq!(spacing.xs, 2.0);
        assert_eq!(spacing.sm, 4.0);
        assert_eq!(spacing.md, 8.0);
        assert_eq!(spacing.lg, 16.0);
        assert_eq!(spacing.xl, 24.0);
        assert_eq!(spacing.xxl, 32.0);
    }

    #[test]
    fn test_theme_colors_to_visuals_light() {
        let colors = ThemeColors::light();
        let visuals = colors.to_visuals();

        // Light theme visuals should not be dark mode
        assert!(!visuals.dark_mode);

        // Panel fill should match our theme's background
        assert_eq!(visuals.panel_fill, colors.base.background);
    }

    #[test]
    fn test_theme_colors_to_visuals_dark() {
        let colors = ThemeColors::dark();
        let visuals = colors.to_visuals();

        // Dark theme visuals should be dark mode
        assert!(visuals.dark_mode);

        // Panel fill should match our theme's background
        assert_eq!(visuals.panel_fill, colors.base.background);
    }

    #[test]
    fn test_visuals_for_theme_light() {
        let visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::Light,
            &eframe::egui::Visuals::light(),
            accent::default_accent(),
        );
        assert!(!visuals.dark_mode);
    }

    #[test]
    fn test_visuals_for_theme_dark() {
        let visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::Dark,
            &eframe::egui::Visuals::dark(),
            accent::default_accent(),
        );
        assert!(visuals.dark_mode);
    }

    #[test]
    fn test_visuals_for_theme_system() {
        // System theme follows the provided visuals
        let dark_visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::System,
            &eframe::egui::Visuals::dark(),
            accent::default_accent(),
        );
        assert!(dark_visuals.dark_mode);

        let light_visuals = ThemeColors::visuals_for_theme(
            crate::config::Theme::System,
            &eframe::egui::Visuals::light(),
            accent::default_accent(),
        );
        assert!(!light_visuals.dark_mode);
    }
}
