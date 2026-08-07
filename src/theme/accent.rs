//! User-configurable Ferrite accent (headings, selection tint, UI chrome).
//! Standard hyperlink blues are fixed in [`standard_link_color`].

use eframe::egui::Color32;

/// Default accent — a terracotta that works on both the warm paper and warm
/// charcoal grounds.
///
/// A single value has to serve both themes, because [`ThemeColors::apply_user_accent`]
/// applies the user's one choice to whichever theme is active. Measured:
/// 4.23:1 on the light background, 3.87:1 on the dark one — both clear the 3:1
/// floor for UI components, and [`readable_on`] lifts it further where it is
/// used as document text.
///
/// The previous value was a cool blue tuned for dark chrome; on a light page it
/// measured 2.2:1 as a heading colour.
pub const DEFAULT_ACCENT_RGB: [u8; 3] = [188, 92, 54];

#[inline]
pub fn default_accent() -> Color32 {
    Color32::from_rgb(
        DEFAULT_ACCENT_RGB[0],
        DEFAULT_ACCENT_RGB[1],
        DEFAULT_ACCENT_RGB[2],
    )
}

/// Classic link blues (not controlled by accent).
#[inline]
pub fn standard_link_color(dark_mode: bool) -> Color32 {
    if dark_mode {
        Color32::from_rgb(100, 180, 255)
    } else {
        Color32::from_rgb(0, 90, 170)
    }
}

#[inline]
fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    ((f32::from(a)) * (1.0 - t) + (f32::from(b)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        lerp_channel(a.r(), b.r(), t),
        lerp_channel(a.g(), b.g(), t),
        lerp_channel(a.b(), b.b(), t),
    )
}

pub fn accent_hover(accent: Color32, dark: bool) -> Color32 {
    if dark {
        lerp_color(accent, Color32::WHITE, 0.12)
    } else {
        lerp_color(accent, Color32::BLACK, 0.15)
    }
}

/// WCAG relative luminance of a color, in the range `0.0..=1.0`.
///
/// Uses the sRGB transfer function rather than a naive channel average — the
/// two disagree sharply on saturated blues and greens, which is exactly where
/// foreground choices go wrong.
fn relative_luminance(color: Color32) -> f32 {
    #[inline]
    fn channel(c: u8) -> f32 {
        let c = f32::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

/// WCAG contrast ratio between two colors, from 1.0 (identical) to 21.0.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Near-black foreground used on light accents.
const ON_ACCENT_DARK: Color32 = Color32::from_rgb(20, 20, 20);
/// Near-white foreground used on dark accents.
const ON_ACCENT_LIGHT: Color32 = Color32::from_rgb(250, 250, 250);

/// WCAG AA floor for body text.
pub const MIN_TEXT_CONTRAST: f32 = 4.5;

/// Darken or lighten `color` until it reaches `min_ratio` against `background`,
/// keeping as much of the original hue as possible.
///
/// This exists because an accent has two incompatible jobs. As UI chrome it
/// sits on panel fills and wants to be vivid; as *document text* — a heading on
/// the page — it has to be readable. The default accent is a light blue that
/// measures 7.6:1 on the dark panel background but only 2.2:1 on a white page,
/// so using it raw for headings fails even the 3:1 large-text floor.
///
/// Blends toward black or white (whichever direction the background calls for)
/// and returns the first step that clears `min_ratio`, so a well-chosen accent
/// is returned untouched and only an unreadable one is corrected.
pub fn readable_on(color: Color32, background: Color32, min_ratio: f32) -> Color32 {
    if contrast_ratio(color, background) >= min_ratio {
        return color;
    }

    // Move away from the background: darken on light pages, lighten on dark.
    let toward = if relative_luminance(background) > 0.5 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };

    // Contrast is monotonic in t along this blend, so bisect for the smallest
    // adjustment that clears the floor rather than jumping straight to
    // black/white and throwing the user's hue away.
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    for _ in 0..16 {
        let mid = (lo + hi) / 2.0;
        if contrast_ratio(lerp_color(color, toward, mid), background) >= min_ratio {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let adjusted = lerp_color(color, toward, hi);

    // Rounding to u8 can land a hair under the target; fall back to the
    // extreme rather than return something that still fails.
    if contrast_ratio(adjusted, background) >= min_ratio {
        adjusted
    } else {
        toward
    }
}

/// Foreground color to place on top of `accent`.
///
/// The default accent is a light blue; white text on it measures 2.2:1, below
/// the 3:1 floor even for large text.
///
/// This measures both candidates and returns whichever actually contrasts
/// better, rather than thresholding on luminance. A threshold is easy to get
/// wrong: the crossover between these two foregrounds sits at a luminance of
/// ~0.19, so an intuitive-looking cutoff of 0.5 would pick light text for
/// mid-luminance accents at ratios as poor as 2.2:1 — reintroducing the very
/// bug this function exists to fix.
///
/// Contrast peaks at the extremes and bottoms out around 4.2:1 for accents
/// near the crossover, so this guarantees the best available choice, not that
/// every accent clears 4.5:1. Accents are user-chosen and unconstrained in the
/// welcome screen's color picker, so that floor cannot be guaranteed here.
pub fn on_accent(accent: Color32) -> Color32 {
    if contrast_ratio(accent, ON_ACCENT_DARK) >= contrast_ratio(accent, ON_ACCENT_LIGHT) {
        ON_ACCENT_DARK
    } else {
        ON_ACCENT_LIGHT
    }
}

/// egui selection / “open” widget fill derived from accent.
pub fn selection_fill(accent: Color32, dark: bool) -> Color32 {
    if dark {
        let bg = Color32::from_rgb(30, 30, 30);
        lerp_color(bg, accent, 0.42)
    } else {
        lerp_color(Color32::WHITE, accent, 0.28)
    }
}

/// Outline / sidebar highlights (muted vs full selection_fill).
pub fn panel_highlight_fill(
    panel_bg: Color32,
    accent: Color32,
    dark: bool,
    strength: f32,
) -> Color32 {
    let t = if dark { strength } else { strength * 0.55 };
    lerp_color(panel_bg, accent, t.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-good reference values from the WCAG 2.x definition.
    #[test]
    fn relative_luminance_matches_wcag_reference() {
        assert!((relative_luminance(Color32::BLACK) - 0.0).abs() < 1e-4);
        assert!((relative_luminance(Color32::WHITE) - 1.0).abs() < 1e-4);
        // Pure sRGB primaries are the coefficients themselves.
        assert!((relative_luminance(Color32::from_rgb(255, 0, 0)) - 0.2126).abs() < 1e-3);
        assert!((relative_luminance(Color32::from_rgb(0, 255, 0)) - 0.7152).abs() < 1e-3);
        assert!((relative_luminance(Color32::from_rgb(0, 0, 255)) - 0.0722).abs() < 1e-3);
    }

    #[test]
    fn contrast_ratio_is_symmetric_and_bounded() {
        let ratio = contrast_ratio(Color32::BLACK, Color32::WHITE);
        assert!((ratio - 21.0).abs() < 0.05, "black on white should be 21:1");
        assert!((contrast_ratio(Color32::WHITE, Color32::BLACK) - ratio).abs() < 1e-4);
        assert!((contrast_ratio(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 1e-4);
    }

    /// A label drawn on the accent must clear the 3:1 floor for UI components
    /// and large text.
    ///
    /// Deliberately NOT `MIN_TEXT_CONTRAST`. One accent has to serve both a
    /// light and a dark ground, which pins it near mid-luminance, and from
    /// there contrast is limited in every direction: a search over the whole
    /// warm-red family maxes out at ~4.05:1 for
    /// `min(vs light bg, vs dark bg, vs its own label)`. 4.5:1 is unreachable
    /// for any dual-ground accent, not just this one.
    ///
    /// Small text therefore belongs on `base.selected` — the blended selection
    /// fill, which measures 11.3:1 (light) and 8.8:1 (dark) against primary
    /// text — not on the raw accent. See `theme::contrast_tests`.
    #[test]
    fn default_accent_gets_a_readable_foreground() {
        let accent = default_accent();
        let fg = on_accent(accent);
        let ratio = contrast_ratio(accent, fg);
        assert!(
            ratio >= 3.0,
            "on_accent({accent:?}) -> {fg:?} = {ratio:.2}:1, below the 3:1 UI floor"
        );
    }

    /// The bug this exists for: an accent tuned as chrome, used raw as heading
    /// text on a page. Uses an explicitly unreadable colour rather than the
    /// default accent, so the test keeps testing the mechanism even when the
    /// default changes.
    #[test]
    fn readable_on_fixes_an_unreadable_accent_and_keeps_its_hue() {
        let white = Color32::WHITE;
        let washed_out_blue = Color32::from_rgb(100, 180, 255);
        assert!(
            contrast_ratio(washed_out_blue, white) < 3.0,
            "guard: this really is unreadable on a white page"
        );
        let fixed = readable_on(washed_out_blue, white, MIN_TEXT_CONTRAST);
        assert!(contrast_ratio(fixed, white) >= MIN_TEXT_CONTRAST);
        assert!(fixed.b() > fixed.r(), "hue should be preserved");
    }

    /// The shipped default must already be usable as heading text on both
    /// grounds without correction doing heavy lifting.
    #[test]
    fn default_accent_is_usable_on_both_page_grounds() {
        for page in [Color32::from_rgb(251, 249, 245), Color32::from_rgb(28, 27, 25)] {
            let heading = readable_on(default_accent(), page, MIN_TEXT_CONTRAST);
            assert!(contrast_ratio(heading, page) >= MIN_TEXT_CONTRAST);
            assert!(
                contrast_ratio(default_accent(), page) >= 3.0,
                "the accent itself should clear the UI-component floor on {page:?}"
            );
        }
    }

    /// An accent that already reads well must be returned untouched, so this
    /// never silently overrides a good choice.
    #[test]
    fn readable_on_is_identity_when_contrast_already_passes() {
        let white = Color32::WHITE;
        let already_dark = Color32::from_rgb(0, 90, 165);
        assert!(contrast_ratio(already_dark, white) >= MIN_TEXT_CONTRAST);
        assert_eq!(readable_on(already_dark, white, MIN_TEXT_CONTRAST), already_dark);
    }

    /// Must work in both directions — on a dark page it lightens instead.
    #[test]
    fn readable_on_lightens_against_a_dark_background() {
        let page = Color32::from_rgb(30, 30, 30);
        let too_dark = Color32::from_rgb(0, 40, 90);
        let fixed = readable_on(too_dark, page, MIN_TEXT_CONTRAST);
        assert!(contrast_ratio(fixed, page) >= MIN_TEXT_CONTRAST);
        assert!(relative_luminance(fixed) > relative_luminance(too_dark));
    }

    /// Whatever accent the user picks in the unconstrained color picker, the
    /// heading colour derived from it must clear AA on both page backgrounds.
    #[test]
    fn readable_on_clears_aa_for_every_accent_on_both_pages() {
        for page in [Color32::WHITE, Color32::from_rgb(30, 30, 30)] {
            for r in (0..=255).step_by(17) {
                for g in (0..=255).step_by(17) {
                    for b in (0..=255).step_by(17) {
                        let accent = Color32::from_rgb(r as u8, g as u8, b as u8);
                        let fixed = readable_on(accent, page, MIN_TEXT_CONTRAST);
                        let got = contrast_ratio(fixed, page);
                        assert!(
                            got >= MIN_TEXT_CONTRAST - 0.01,
                            "{accent:?} on {page:?} -> {fixed:?} = {got:.2}:1"
                        );
                    }
                }
            }
        }
    }

    /// `on_accent` must always pick the better of its two candidates, across
    /// the whole color space — a luminance threshold would fail this.
    #[test]
    fn on_accent_always_picks_the_higher_contrast_option() {
        let mut worst = f32::MAX;
        for r in (0..=255).step_by(15) {
            for g in (0..=255).step_by(15) {
                for b in (0..=255).step_by(15) {
                    let accent = Color32::from_rgb(r as u8, g as u8, b as u8);
                    let chosen = contrast_ratio(accent, on_accent(accent));
                    let other = contrast_ratio(accent, ON_ACCENT_DARK)
                        .max(contrast_ratio(accent, ON_ACCENT_LIGHT));
                    assert!(
                        (chosen - other).abs() < 1e-5,
                        "{accent:?}: chose {chosen:.2}:1 over {other:.2}:1"
                    );
                    worst = worst.min(chosen);
                }
            }
        }
        // Mid-luminance accents cap out around 4.2:1 whichever foreground is
        // used; assert we never fall below the 3:1 large-text floor.
        assert!(worst >= 3.0, "worst achievable contrast was {worst:.2}:1");
    }
}
