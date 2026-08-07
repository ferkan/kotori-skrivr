//! Regression tests that keep the warm palette in `theme::mod` from silently
//! drifting below WCAG AA. These assert the *actual* rendered pairings —
//! text on background, text on selection fill, heading after the accent is
//! applied — rather than trusting the doc-comment numbers next to the
//! constants.

use super::{ThemeColors, accent};
use eframe::egui::Color32;

/// One contrast assertion: a named foreground/background pairing and the
/// WCAG floor it must clear.
struct ContrastCase {
    label: String,
    fg: Color32,
    bg: Color32,
    floor: f32,
}

/// Check every case, reporting the token name, both colors, the measured
/// ratio and the floor on failure — not a bare `assert!`.
fn assert_contrast_floors(cases: &[ContrastCase]) {
    for case in cases {
        let ratio = accent::contrast_ratio(case.fg, case.bg);
        assert!(
            ratio >= case.floor,
            "{} ({},{},{}) on ({},{},{}) = {:.2}:1, floor {:.1}:1",
            case.label,
            case.fg.r(),
            case.fg.g(),
            case.fg.b(),
            case.bg.r(),
            case.bg.g(),
            case.bg.b(),
            ratio,
            case.floor,
        );
    }
}

/// The standard set of token-vs-background/selection floors, parameterized
/// by theme name and palette so light/dark share one case list.
fn base_cases(theme: &'static str, colors: &ThemeColors) -> Vec<ContrastCase> {
    let bg = colors.base.background;
    vec![
        ContrastCase {
            label: format!("{theme} text.primary"),
            fg: colors.text.primary,
            bg,
            floor: 4.5,
        },
        ContrastCase {
            label: format!("{theme} text.secondary"),
            fg: colors.text.secondary,
            bg,
            floor: 4.5,
        },
        ContrastCase {
            label: format!("{theme} text.muted"),
            fg: colors.text.muted,
            bg,
            floor: 4.5,
        },
        ContrastCase {
            label: format!("{theme} text.disabled"),
            fg: colors.text.disabled,
            bg,
            floor: 3.0,
        },
        ContrastCase {
            label: format!("{theme} text.link"),
            fg: colors.text.link,
            bg,
            floor: 4.5,
        },
        ContrastCase {
            // A security notice the user must read before enabling code
            // execution. The light amber measured 1.55:1 before this was
            // guarded.
            label: format!("{theme} ui.warning"),
            fg: colors.ui.warning,
            bg,
            floor: 4.5,
        },
        ContrastCase {
            label: format!("{theme} base.border"),
            fg: colors.base.border,
            bg,
            floor: 3.0,
        },
        ContrastCase {
            label: format!("{theme} text.primary on base.selected"),
            fg: colors.text.primary,
            bg: colors.base.selected,
            floor: 4.5,
        },
        ContrastCase {
            label: format!("{theme} editor.blockquote_text"),
            fg: colors.editor.blockquote_text,
            bg,
            floor: 4.5,
        },
    ]
}

#[test]
fn light_palette_meets_contrast_floors() {
    let colors = ThemeColors::light();
    assert_contrast_floors(&base_cases("light", &colors));

    let mut with_accent = ThemeColors::light();
    with_accent.apply_user_accent(accent::default_accent());
    assert_contrast_floors(&[ContrastCase {
        label: "light editor.heading (after apply_user_accent)".to_string(),
        fg: with_accent.editor.heading,
        bg: with_accent.base.background,
        floor: 4.5,
    }]);
}

#[test]
fn dark_palette_meets_contrast_floors() {
    let colors = ThemeColors::dark();
    assert_contrast_floors(&base_cases("dark", &colors));

    let mut with_accent = ThemeColors::dark();
    with_accent.apply_user_accent(accent::default_accent());
    assert_contrast_floors(&[ContrastCase {
        label: "dark editor.heading (after apply_user_accent)".to_string(),
        fg: with_accent.editor.heading,
        bg: with_accent.base.background,
        floor: 4.5,
    }]);
}

/// A hairline divider (`border_subtle`) should read as the same visual
/// weight in both themes, not accidentally near-invisible in one of them.
#[test]
fn border_subtle_reads_the_same_weight_in_both_themes() {
    let light = ThemeColors::light();
    let dark = ThemeColors::dark();

    let light_ratio = accent::contrast_ratio(light.base.border_subtle, light.base.background);
    let dark_ratio = accent::contrast_ratio(dark.base.border_subtle, dark.base.background);
    let diff = (light_ratio - dark_ratio).abs();

    assert!(
        diff <= 0.15,
        "border_subtle weight mismatch: light = {light_ratio:.2}:1, dark = {dark_ratio:.2}:1, diff {diff:.2} > 0.15"
    );
}
