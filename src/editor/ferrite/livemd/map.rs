//! `LineMap`: source column <-> display column, in byte offsets.
//!
//! No egui types. The buffer, undo history, save and search always operate
//! in source columns; only rendering and hit-testing touch display columns
//! (see `.claude/skills/livemd/SKILL.md`). Every public function here is
//! total: it never panics and never returns an index that lands mid-UTF-8
//! sequence, for any input including out-of-range columns.

use super::{SpanRole, StyleSpan};

/// One tile of the source line: a contiguous byte range, and where (if
/// anywhere) it appears in the display string. `hidden` segments (Marker
/// spans) collapse to a single zero-width point in display space.
#[derive(Debug, Clone, Copy)]
struct Seg {
    src_start: usize,
    src_end: usize,
    disp_start: usize,
    disp_end: usize,
    hidden: bool,
}

/// Maps between source-line byte columns and display-line byte columns for
/// one line, in one reveal state.
///
/// Build with [`LineMap::from_spans`] for a *hidden* line (markers omitted
/// from display) or [`LineMap::identity`] for a *revealed* line (source ==
/// display, per the reveal policy in the contract).
#[derive(Debug, Clone)]
pub struct LineMap {
    segments: Vec<Seg>,
    source_line: String,
    source_len: usize,
    display_len: usize,
    display_string: String,
}

/// Snaps `i` down to the nearest valid char boundary of `s` (or 0). Used so
/// that a caller passing an off-boundary byte offset — which should never
/// happen from cursor code, but this module must be total regardless —
/// still gets a char-boundary-safe result instead of a slicing panic.
fn floor_char_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

impl LineMap {
    /// Builds a map from a line's source text and its tiling `StyleSpan`s
    /// (as produced by `scan::scan_line`). Marker spans are hidden: they
    /// contribute no bytes to the display string and collapse to a single
    /// display column.
    pub fn from_spans(line: &str, spans: &[StyleSpan]) -> Self {
        let mut segments = Vec::with_capacity(spans.len());
        let mut display_string = String::new();
        let mut disp_pos = 0usize;

        for span in spans {
            let src_start = span.range.start;
            let src_end = span.range.end.min(line.len()).max(src_start);
            match span.role {
                SpanRole::Marker => {
                    segments.push(Seg {
                        src_start,
                        src_end,
                        disp_start: disp_pos,
                        disp_end: disp_pos,
                        hidden: true,
                    });
                }
                SpanRole::Text => {
                    if let Some(piece) = line.get(src_start..src_end) {
                        display_string.push_str(piece);
                    }
                    let disp_end = disp_pos + (src_end - src_start);
                    segments.push(Seg {
                        src_start,
                        src_end,
                        disp_start: disp_pos,
                        disp_end,
                        hidden: false,
                    });
                    disp_pos = disp_end;
                }
            }
        }

        LineMap {
            segments,
            source_line: line.to_string(),
            source_len: line.len(),
            display_len: disp_pos,
            display_string,
        }
    }

    /// Identity map for a revealed line: display text equals source text,
    /// column-for-column.
    pub fn identity(line: &str) -> Self {
        LineMap {
            segments: vec![Seg {
                src_start: 0,
                src_end: line.len(),
                disp_start: 0,
                disp_end: line.len(),
                hidden: false,
            }],
            source_line: line.to_string(),
            source_len: line.len(),
            display_len: line.len(),
            display_string: line.to_string(),
        }
    }

    /// The line as it should be rendered: markers omitted (identity for a
    /// revealed-line map).
    pub fn display_string(&self) -> &str {
        &self.display_string
    }

    /// Maps a source byte column to a display byte column. Total: columns
    /// past the end of the line clamp to the end of the display string.
    /// Columns inside a hidden marker's byte range collapse to the single
    /// display point where that marker used to be (many-to-one — this is a
    /// projection, not injective, by design: hidden text has no display
    /// position of its own to distinguish between).
    pub fn to_display(&self, src_col: usize) -> usize {
        let src_col = floor_char_boundary(&self.source_line, src_col);
        for seg in &self.segments {
            if src_col <= seg.src_end && src_col >= seg.src_start {
                return if seg.hidden {
                    seg.disp_start
                } else {
                    seg.disp_start + (src_col - seg.src_start)
                };
            }
        }
        self.display_len
    }

    /// Maps a display byte column to a source byte column. Total.
    ///
    /// Hidden-run rule: a display column that sits exactly at the collapsed
    /// point of a hidden marker run resolves to the **start** of that
    /// source run (never mid-marker, never its end), so that a click where
    /// e.g. `**` used to be places the caret immediately before the marker
    /// and arrowing left from there reveals the line. When several hidden
    /// runs collapse to the same display point (e.g. a blockquote marker
    /// immediately followed by a bold-open marker with no visible text
    /// between them), the earliest one in source order wins.
    pub fn to_source(&self, disp_col: usize) -> usize {
        let disp_col = floor_char_boundary(&self.display_string, disp_col);

        for seg in &self.segments {
            if seg.hidden && seg.disp_start == disp_col {
                return seg.src_start;
            }
        }
        for seg in &self.segments {
            if !seg.hidden && disp_col <= seg.disp_end && disp_col >= seg.disp_start {
                return seg.src_start + (disp_col - seg.disp_start);
            }
        }
        self.source_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::block::BlockContext;
    use super::super::scan::scan_line;

    fn map_for(line: &str) -> LineMap {
        let spans = scan_line(line, BlockContext::Normal);
        LineMap::from_spans(line, &spans)
    }

    fn is_char_boundary_safe(s: &str, col: usize) -> bool {
        col <= s.len() && s.is_char_boundary(col)
    }

    #[test]
    fn identity_map_is_columnwise_identity() {
        let line = "hello world";
        let map = LineMap::identity(line);
        for c in 0..=line.len() {
            assert_eq!(map.to_display(c), c);
            assert_eq!(map.to_source(c), c);
        }
        assert_eq!(map.display_string(), line);
    }

    #[test]
    fn bold_line_display_string_omits_markers() {
        let line = "some **bold** text";
        let map = map_for(line);
        assert_eq!(map.display_string(), "some bold text");
    }

    /// The invariant that actually governs on-screen cursor drift.
    ///
    /// `to_source` is deliberately *not* invertible from the source side: a
    /// hidden marker run occupies real source bytes but zero display width, so
    /// several source columns share one display column and information is lost
    /// (see `text_start_after_hidden_marker_does_not_round_trip`). That is
    /// inherent, not a defect.
    ///
    /// The *display* side, however, must round-trip exactly, and this is the
    /// direction the user can see: click at display column `d`, and the caret
    /// must be drawn back at display column `d`. If this ever fails, the caret
    /// renders somewhere other than where it was clicked — which is precisely
    /// the drift bug upstream still has open against their block-based mode.
    #[test]
    fn display_side_round_trip_is_exact() {
        let corpus = [
            "some **bold** text after",
            "**leading bold** then text",
            "trailing text then **bold**",
            "***both*** and ~~struck~~ and `code`",
            "# heading with **bold** inside",
            "> quoted **bold** line",
            "- list item with *italic*",
            "[link text](https://example.com) after",
            "日本語の**太字**テキスト",
            "emoji 👨‍👩‍👧 with **bold** after",
            "**",
            "**unclosed bold",
            "",
        ];

        for line in corpus {
            let map = map_for(line);
            let disp = map.display_string().to_string();
            for d in 0..=disp.len() {
                if !disp.is_char_boundary(d) {
                    continue;
                }
                let src = map.to_source(d);
                assert!(
                    is_char_boundary_safe(line, src),
                    "to_source({d}) = {src} is not a char boundary of {line:?}"
                );
                assert_eq!(
                    map.to_display(src),
                    d,
                    "display round-trip broke on {line:?}: \
                     display {d} -> source {src} -> display {}",
                    map.to_display(src)
                );
            }
        }
    }

    /// `to_source` must never move backwards as the display column advances.
    /// Non-monotonicity would let a rightward arrow-key press move the caret
    /// left in the buffer.
    #[test]
    fn to_source_is_monotonic_non_decreasing() {
        for line in [
            "some **bold** text after",
            "***nested*** emphasis `code` end",
            "> - **quoted bold list**",
            "日本語の**太字**テキスト",
        ] {
            let map = map_for(line);
            let disp = map.display_string().to_string();
            let mut prev = 0usize;
            for d in 0..=disp.len() {
                if !disp.is_char_boundary(d) {
                    continue;
                }
                let src = map.to_source(d);
                assert!(
                    src >= prev,
                    "to_source went backwards on {line:?}: display {d} gave {src}, previous {prev}"
                );
                prev = src;
            }
        }
    }

    #[test]
    fn round_trip_identity_over_full_corpus() {
        let corpus = [
            "hello world",
            "plain text with no markup at all",
            "line with **bold** and *italic* and `code`",
            "粗体と日本語のテキスト",
            "emoji 👨‍👩‍👧 family and e\u{0301} combining",
        ];
        for line in corpus {
            let map = LineMap::identity(line);
            for c in 0..=line.len() {
                if !line.is_char_boundary(c) {
                    continue;
                }
                let d = map.to_display(c);
                assert!(is_char_boundary_safe(line, d));
                let back = map.to_source(d);
                assert_eq!(back, c, "round trip failed for identity map at col {c} in {line:?}");
            }
        }
    }

    #[test]
    fn round_trip_holds_on_text_span_interiors_of_hidden_map() {
        // Round trip holds for source columns strictly *inside* a Text
        // span's byte range (including its end, which is either the line
        // end or the start of the next span). It deliberately does NOT
        // hold at the *first* byte of a Text span when that span is
        // immediately preceded by a hidden Marker span: that shared
        // display point resolves, by the hidden-run-snap rule, to the
        // marker's source start rather than back to the text's own start.
        // See `text_start_after_hidden_marker_does_not_round_trip` below —
        // this is a documented, load-bearing conflict between the
        // contract's "round trip for every source column" test expectation
        // and its "snap to hidden-run start" mapping rule, not a bug.
        let line = "some **bold** text after";
        let spans = scan_line(line, BlockContext::Normal);
        let map = LineMap::from_spans(line, &spans);

        let mut prev_hidden = false;
        for span in &spans {
            let is_hidden = span.role == super::SpanRole::Marker;
            if !is_hidden {
                for c in span.range.clone() {
                    if !line.is_char_boundary(c) {
                        continue;
                    }
                    if c == span.range.start && prev_hidden {
                        continue; // documented exception, see above
                    }
                    let d = map.to_display(c);
                    assert!(is_char_boundary_safe(map.display_string(), d));
                    let back = map.to_source(d);
                    assert_eq!(back, c, "round trip failed at text col {c} in {line:?}");
                }
            }
            prev_hidden = is_hidden;
        }
    }

    #[test]
    fn text_start_after_hidden_marker_does_not_round_trip() {
        // Documents the conflict: col 7 is the first byte of the Text span
        // "bold", immediately preceded by the hidden "**" marker (5..7).
        // to_display(7) == 5 (the marker's collapsed display point, which
        // coincides with where "bold" starts in the display string), and
        // to_source(5) == 5 (snaps to the marker's start per the hidden-run
        // rule) rather than back to 7. Universal round-trip and
        // snap-to-hidden-run-start cannot both hold at this boundary.
        let line = "some **bold** text after";
        let spans = scan_line(line, BlockContext::Normal);
        let map = LineMap::from_spans(line, &spans);
        assert_eq!(map.to_display(7), 5);
        assert_eq!(map.to_source(5), 5);
    }

    #[test]
    fn hidden_marker_interior_collapses_to_marker_start() {
        // "some **bold** text": the opening marker "**" spans byte 5..7.
        let line = "some **bold** text";
        let spans = scan_line(line, BlockContext::Normal);
        let map = LineMap::from_spans(line, &spans);

        // Every column inside the marker's source range maps to the same
        // display point...
        let d5 = map.to_display(5);
        let d6 = map.to_display(6);
        assert_eq!(d5, d6);
        // ...and the reverse mapping snaps to the marker's start, not to
        // whatever interior column originally produced that display point.
        assert_eq!(map.to_source(d5), 5);
    }

    #[test]
    fn click_at_collapsed_marker_point_snaps_to_marker_start() {
        let line = "some **bold** text";
        let map = map_for(line);
        // Display string: "some bold text"; the collapsed point for the
        // opening "**" sits right after "some " (display col 5).
        assert_eq!(map.display_string(), "some bold text");
        assert_eq!(map.to_source(5), 5); // source col of "**" start
    }

    #[test]
    fn adjacent_hidden_runs_collapse_to_earliest_start() {
        // Blockquote marker directly followed by a bold-open marker with
        // no visible text between them: "> **bold**"
        let line = "> **bold**";
        let map = map_for(line);
        assert_eq!(map.display_string(), "bold");
        // Both hidden runs collapse to display col 0; earliest wins (the
        // very start of the line).
        assert_eq!(map.to_source(0), 0);
    }

    #[test]
    fn cjk_and_emoji_no_panic_and_char_boundary_safe() {
        let line = "**粗体** と *斜体* 👨‍👩‍👧 e\u{0301}nd";
        let map = map_for(line);
        for c in 0..=line.len() {
            let d = map.to_display(c);
            assert!(is_char_boundary_safe(map.display_string(), d), "to_display({c}) -> {d} not on boundary");
            let s = map.to_source(d);
            assert!(s <= line.len() && line.is_char_boundary(s), "to_source({d}) -> {s} not on boundary");
        }
    }

    #[test]
    fn out_of_range_columns_clamp_instead_of_panicking() {
        let line = "short";
        let map = map_for(line);
        assert_eq!(map.to_display(9999), map.display_string().len());
        assert_eq!(map.to_source(9999), line.len());
    }

    #[test]
    fn empty_line_map() {
        let map = map_for("");
        assert_eq!(map.display_string(), "");
        assert_eq!(map.to_display(0), 0);
        assert_eq!(map.to_source(0), 0);
    }

    #[test]
    fn fully_hidden_line_collapses_to_empty_display() {
        // A line that is a heading marker only, no text: "###"
        let line = "###";
        let map = map_for(line);
        assert_eq!(map.display_string(), "");
        assert_eq!(map.to_display(0), 0);
        assert_eq!(map.to_display(3), 0);
        assert_eq!(map.to_source(0), 0);
    }
}
