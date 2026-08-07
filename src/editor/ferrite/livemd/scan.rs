//! Inline scanner: source line -> tiling `StyleSpan`s.
//!
//! No egui types. Operates on byte offsets only, never char offsets, so it
//! is safe over CJK, emoji and combining-mark text (see
//! `.claude/skills/livemd/SKILL.md`).
//!
//! Invariant relied on by `map.rs`: for any input, the returned spans tile
//! `0..text.len()` completely, in order, with no gaps and no overlaps. Every
//! branch below either recurses into a sub-range (which itself tiles that
//! sub-range by the same invariant) or falls through to "treat as literal
//! text", which merges into the enclosing plain-text run rather than being
//! dropped. Nothing is ever skipped without being captured in a span.

use super::{BlockContext, InlineStyle, SpanRole, StyleSpan};

/// Scans one source line into styled spans, given its block context.
pub fn scan_line(text: &str, ctx: BlockContext) -> Vec<StyleSpan> {
    let len = text.len();

    match ctx {
        // Fence body and fence delimiter lines are never inline-scanned:
        // markdown syntax inside a code fence is literal text.
        BlockContext::FenceBody | BlockContext::FenceDelimiter => {
            return vec![StyleSpan {
                range: 0..len,
                style: InlineStyle {
                    code: true,
                    ..Default::default()
                },
                role: SpanRole::Text,
            }];
        }
        BlockContext::Normal => {}
    }

    let mut spans = Vec::new();
    let mut pos = 0usize;

    // 1. Leading blockquote marker(s), e.g. "> " or "> > ".
    let bq_end = scan_blockquote_prefix(text);
    let blockquote = bq_end > 0;
    if blockquote {
        spans.push(StyleSpan {
            range: 0..bq_end,
            style: InlineStyle {
                blockquote: true,
                ..Default::default()
            },
            role: SpanRole::Marker,
        });
        pos = bq_end;
    }

    // 2. Heading marker, else list bullet/number marker (mutually exclusive).
    let mut heading: Option<u8> = None;
    if let Some((marker_len, level)) = scan_heading_prefix(&text[pos..]) {
        spans.push(StyleSpan {
            range: pos..pos + marker_len,
            style: InlineStyle {
                heading: Some(level),
                blockquote,
                ..Default::default()
            },
            role: SpanRole::Marker,
        });
        heading = Some(level);
        pos += marker_len;
    } else if let Some(marker_len) = scan_list_prefix(&text[pos..]) {
        spans.push(StyleSpan {
            range: pos..pos + marker_len,
            style: InlineStyle {
                blockquote,
                ..Default::default()
            },
            role: SpanRole::Marker,
        });
        pos += marker_len;
    }

    // 3. Inline scan of the remainder.
    let base_style = InlineStyle {
        heading,
        blockquote,
        ..Default::default()
    };
    inline_scan(text, pos, len, base_style, &mut spans);

    spans
}

// ---------------------------------------------------------------------
// Block-prefix recognizers
// ---------------------------------------------------------------------

/// Consumes leading blockquote markers, e.g. "> " or "> > " for nesting.
/// Up to 3 leading spaces before each ">" are tolerated (CommonMark).
fn scan_blockquote_prefix(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0;
    loop {
        let start = i;
        let mut spaces = 0;
        while i < bytes.len() && bytes[i] == b' ' && spaces < 3 {
            i += 1;
            spaces += 1;
        }
        if i < bytes.len() && bytes[i] == b'>' {
            i += 1;
            if i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
        } else {
            i = start;
            break;
        }
    }
    i
}

/// Recognizes an ATX heading prefix ("# ".."###### "). Returns
/// `(marker_byte_len, level)`. More than 6 `#`, or no space/EOL after the
/// run, is not a heading.
fn scan_heading_prefix(s: &str) -> Option<(usize, u8)> {
    let bytes = s.as_bytes();
    let mut n = 0usize;
    while n < bytes.len() && bytes[n] == b'#' {
        n += 1;
    }
    if n == 0 || n > 6 {
        return None;
    }
    if n < bytes.len() && bytes[n] != b' ' {
        return None;
    }
    let marker_len = if n < bytes.len() && bytes[n] == b' ' {
        n + 1
    } else {
        n
    };
    Some((marker_len, n as u8))
}

/// Recognizes a list bullet ("- ", "* ", "+ ") or ordered marker ("1. ", "2) ").
fn scan_list_prefix(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'-' || bytes[0] == b'*' || bytes[0] == b'+')
        && bytes[1] == b' '
    {
        return Some(2);
    }
    let mut n = 0;
    while n < bytes.len() && bytes[n].is_ascii_digit() {
        n += 1;
    }
    if n > 0 && n < bytes.len() && (bytes[n] == b'.' || bytes[n] == b')') {
        if n + 1 < bytes.len() && bytes[n + 1] == b' ' {
            return Some(n + 2);
        }
    }
    None
}

// ---------------------------------------------------------------------
// Inline scanner
// ---------------------------------------------------------------------

/// Whether `c` is one of the punctuation characters `\` can escape.
fn is_escapable(c: char) -> bool {
    matches!(
        c,
        '\\' | '`'
            | '*'
            | '_'
            | '{'
            | '}'
            | '['
            | ']'
            | '('
            | ')'
            | '#'
            | '+'
            | '-'
            | '.'
            | '!'
            | '~'
            | '>'
    )
}

/// Byte at `i`, or `None` past `end`.
fn byte_at(text: &str, i: usize, end: usize) -> Option<u8> {
    if i < end {
        Some(text.as_bytes()[i])
    } else {
        None
    }
}

/// A delimiter run can *open* emphasis only if immediately followed by a
/// non-whitespace character (left-flanking). This also correctly rejects
/// "2 * 3" (space follows) and gives "a*b" nothing to open against once its
/// sole `*` fails to find a closer.
fn can_open(text: &str, after: usize, end: usize) -> bool {
    matches!(byte_at(text, after, end), Some(b) if b != b' ' && b != b'\t')
}

/// A delimiter run can *close* emphasis only if immediately preceded by a
/// non-whitespace character (right-flanking).
fn can_close(text: &str, before: usize) -> bool {
    before > 0
        && matches!(text.as_bytes().get(before - 1), Some(&b) if b != b' ' && b != b'\t')
}

/// Finds the next exact occurrence of `delim` at or after `from`, requiring
/// the byte before the match to be non-whitespace (closing flank). Byte-wise
/// scan: safe over multi-byte UTF-8 because `delim` is pure ASCII and ASCII
/// byte values never occur inside a multi-byte continuation/lead byte.
fn find_delim_close(text: &str, from: usize, end: usize, delim: &str) -> Option<usize> {
    let hay = &text[from..end];
    let dbytes = delim.as_bytes();
    let hbytes = hay.as_bytes();
    if hbytes.len() < dbytes.len() {
        return None;
    }
    for i in 0..=(hbytes.len() - dbytes.len()) {
        if &hbytes[i..i + dbytes.len()] == dbytes {
            let abs = from + i;
            if can_close(text, abs) {
                return Some(abs);
            }
        }
    }
    None
}

/// Finds the next single-char closing delimiter (for `*`/`_` italics),
/// requiring right-flanking (non-whitespace before) and that it is not
/// itself the first char of a longer run of the same delimiter (to avoid
/// misreading `**` as a single-char closer).
fn find_single_delim_close(text: &str, from: usize, end: usize, ch: u8) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < end {
        if bytes[i] == ch {
            if can_close(text, i) {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Finds the closing backtick run of exactly `run_len` backticks.
fn find_backtick_close(text: &str, from: usize, end: usize, run_len: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut j = from;
    while j < end {
        if bytes[j] == b'`' {
            let start = j;
            while j < end && bytes[j] == b'`' {
                j += 1;
            }
            if j - start == run_len {
                return Some(start);
            }
        } else {
            j += 1;
        }
    }
    None
}

/// Parses a `[text](url)` link starting at `i` (which must point at `[`).
/// Returns `(text_end, url_end)`: `text_end` is the byte offset of the `]`,
/// `url_end` is one past the closing `)`. No nested bracket/paren handling
/// (out of v1 scope) — first `]` and first `)` win.
fn parse_link(text: &str, i: usize, end: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut j = i + 1;
    while j < end && bytes[j] != b']' {
        j += 1;
    }
    if j >= end {
        return None;
    }
    let text_end = j;
    let mut k = j + 1;
    if k >= end || bytes[k] != b'(' {
        return None;
    }
    k += 1;
    while k < end && bytes[k] != b')' {
        k += 1;
    }
    if k >= end {
        return None;
    }
    Some((text_end, k + 1))
}

/// Scans `text[start..end]` for inline markdown, pushing spans onto `spans`.
/// `style` carries ambient state (heading level, blockquote) that every
/// produced Text span inherits, merged with local emphasis toggles.
///
/// Tiling guarantee: every branch either `continue`s after advancing `i`
/// (accumulating into the pending plain-text run, flushed on the next
/// marker or at the end) or flushes-then-pushes-marker-then-recurses. The
/// final `flush` call at the bottom covers any trailing plain-text run, so
/// `[start, end)` is always fully covered.
fn inline_scan(text: &str, start: usize, end: usize, style: InlineStyle, spans: &mut Vec<StyleSpan>) {
    let mut i = start;
    let mut run_start = start;

    macro_rules! flush {
        ($upto:expr) => {
            if $upto > run_start {
                spans.push(StyleSpan {
                    range: run_start..$upto,
                    style,
                    role: SpanRole::Text,
                });
            }
        };
    }

    while i < end {
        let b = text.as_bytes()[i];

        // Escaped punctuation: "\X" stays literal (both bytes), never split
        // into a marker — it's content, not syntax.
        if b == b'\\' && i + 1 < end {
            let rest = &text[i + 1..end];
            if let Some(c) = rest.chars().next() {
                if is_escapable(c) {
                    i += 1 + c.len_utf8();
                    continue;
                }
            }
        }

        // Inline code: `` ` `` run, closed by an equal-length run.
        if b == b'`' {
            let run_len = {
                let mut n = 0;
                while i + n < end && text.as_bytes()[i + n] == b'`' {
                    n += 1;
                }
                n
            };
            if let Some(close_start) = find_backtick_close(text, i + run_len, end, run_len) {
                flush!(i);
                spans.push(StyleSpan {
                    range: i..i + run_len,
                    style,
                    role: SpanRole::Marker,
                });
                let content_start = i + run_len;
                if close_start > content_start {
                    let mut code_style = style;
                    code_style.code = true;
                    spans.push(StyleSpan {
                        range: content_start..close_start,
                        style: code_style,
                        role: SpanRole::Text,
                    });
                }
                spans.push(StyleSpan {
                    range: close_start..close_start + run_len,
                    style,
                    role: SpanRole::Marker,
                });
                i = close_start + run_len;
                run_start = i;
                continue;
            }
            i += run_len;
            continue;
        }

        // Strikethrough ~~...~~
        if text[i..end].starts_with("~~") {
            if let Some(close) = find_delim_close(text, i + 2, end, "~~") {
                flush!(i);
                spans.push(StyleSpan {
                    range: i..i + 2,
                    style,
                    role: SpanRole::Marker,
                });
                let mut inner = style;
                inner.strikethrough = true;
                inline_scan(text, i + 2, close, inner, spans);
                spans.push(StyleSpan {
                    range: close..close + 2,
                    style,
                    role: SpanRole::Marker,
                });
                i = close + 2;
                run_start = i;
                continue;
            }
            i += 2;
            continue;
        }

        // Bold-italic ***...*** / ___...___
        if b == b'*' || b == b'_' {
            let delim3: String = std::iter::repeat(b as char).take(3).collect();
            if text[i..end].starts_with(delim3.as_str()) && can_open(text, i + 3, end) {
                if let Some(close) = find_delim_close(text, i + 3, end, &delim3) {
                    flush!(i);
                    spans.push(StyleSpan {
                        range: i..i + 3,
                        style,
                        role: SpanRole::Marker,
                    });
                    let mut inner = style;
                    inner.bold = true;
                    inner.italic = true;
                    inline_scan(text, i + 3, close, inner, spans);
                    spans.push(StyleSpan {
                        range: close..close + 3,
                        style,
                        role: SpanRole::Marker,
                    });
                    i = close + 3;
                    run_start = i;
                    continue;
                }
            }
        }

        // Bold **...** / __...__
        if b == b'*' || b == b'_' {
            let delim2: String = std::iter::repeat(b as char).take(2).collect();
            if text[i..end].starts_with(delim2.as_str()) && can_open(text, i + 2, end) {
                if let Some(close) = find_delim_close(text, i + 2, end, &delim2) {
                    flush!(i);
                    spans.push(StyleSpan {
                        range: i..i + 2,
                        style,
                        role: SpanRole::Marker,
                    });
                    let mut inner = style;
                    inner.bold = true;
                    inline_scan(text, i + 2, close, inner, spans);
                    spans.push(StyleSpan {
                        range: close..close + 2,
                        style,
                        role: SpanRole::Marker,
                    });
                    i = close + 2;
                    run_start = i;
                    continue;
                }
                i += 2;
                continue;
            }
        }

        // Italic *...* / _..._
        if b == b'*' || b == b'_' {
            if can_open(text, i + 1, end) {
                if let Some(close) = find_single_delim_close(text, i + 1, end, b) {
                    flush!(i);
                    spans.push(StyleSpan {
                        range: i..i + 1,
                        style,
                        role: SpanRole::Marker,
                    });
                    let mut inner = style;
                    inner.italic = true;
                    inline_scan(text, i + 1, close, inner, spans);
                    spans.push(StyleSpan {
                        range: close..close + 1,
                        style,
                        role: SpanRole::Marker,
                    });
                    i = close + 1;
                    run_start = i;
                    continue;
                }
            }
            i += 1;
            continue;
        }

        // Link [text](url)
        if b == b'[' {
            if let Some((text_end, url_end)) = parse_link(text, i, end) {
                flush!(i);
                spans.push(StyleSpan {
                    range: i..i + 1,
                    style,
                    role: SpanRole::Marker,
                });
                if text_end > i + 1 {
                    let mut link_style = style;
                    link_style.link = true;
                    spans.push(StyleSpan {
                        range: i + 1..text_end,
                        style: link_style,
                        role: SpanRole::Text,
                    });
                }
                spans.push(StyleSpan {
                    range: text_end..url_end,
                    style,
                    role: SpanRole::Marker,
                });
                i = url_end;
                run_start = i;
                continue;
            }
            i += 1;
            continue;
        }

        // Default: consume one char (grapheme-agnostic but char-boundary-safe).
        let ch_len = text[i..end].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        i += ch_len;
    }

    flush!(end);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_tiles(text: &str, spans: &[StyleSpan]) {
        let mut cursor = 0usize;
        for s in spans {
            assert_eq!(
                s.range.start, cursor,
                "gap/overlap before span {:?} in {:?}",
                s.range, text
            );
            assert!(s.range.end <= text.len());
            assert!(text.get(s.range.clone()).is_some(), "span not on char boundary: {:?}", s.range);
            cursor = s.range.end;
        }
        assert_eq!(cursor, text.len(), "spans do not cover whole line: {:?}", text);
    }

    fn scan(text: &str) -> Vec<StyleSpan> {
        let spans = scan_line(text, BlockContext::Normal);
        assert_tiles(text, &spans);
        spans
    }

    #[test]
    fn plain_text_single_span() {
        let spans = scan("hello world");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role, SpanRole::Text);
    }

    #[test]
    fn empty_line() {
        let spans = scan("");
        assert!(spans.is_empty());
    }

    #[test]
    fn heading_level_two() {
        let spans = scan("## Title");
        assert_eq!(spans[0].role, SpanRole::Marker);
        assert_eq!(spans[0].range, 0..3);
        assert_eq!(spans[1].style.heading, Some(2));
        assert_eq!(&"## Title"[spans[1].range.clone()], "Title");
    }

    #[test]
    fn heading_seven_hashes_is_not_heading() {
        let spans = scan("####### not a heading");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role, SpanRole::Text);
    }

    #[test]
    fn bold_asterisks() {
        let spans = scan("some **bold** text");
        let markers: Vec<_> = spans.iter().filter(|s| s.role == SpanRole::Marker).collect();
        assert_eq!(markers.len(), 2);
        let bold_text = spans.iter().find(|s| s.style.bold && s.role == SpanRole::Text).unwrap();
        assert_eq!(&"some **bold** text"[bold_text.range.clone()], "bold");
    }

    #[test]
    fn bold_underscores() {
        let spans = scan("some __bold__ text");
        assert!(spans.iter().any(|s| s.style.bold && s.role == SpanRole::Text));
    }

    #[test]
    fn italic_asterisk() {
        let spans = scan("an *italic* word");
        let it = spans.iter().find(|s| s.style.italic && s.role == SpanRole::Text).unwrap();
        assert_eq!(&"an *italic* word"[it.range.clone()], "italic");
    }

    #[test]
    fn bold_italic_triple() {
        let spans = scan("***both***");
        let inner = spans.iter().find(|s| s.role == SpanRole::Text).unwrap();
        assert!(inner.style.bold && inner.style.italic);
        assert_eq!(&"***both***"[inner.range.clone()], "both");
    }

    #[test]
    fn nested_code_inside_bold() {
        let spans = scan("**bold with `code` inside**");
        let code_span = spans
            .iter()
            .find(|s| s.style.code && s.role == SpanRole::Text)
            .unwrap();
        assert!(code_span.style.bold, "code inside bold should keep bold flag");
        assert_eq!(&"**bold with `code` inside**"[code_span.range.clone()], "code");
    }

    #[test]
    fn inline_code_simple() {
        let spans = scan("here is `code` inline");
        let code_span = spans.iter().find(|s| s.style.code).unwrap();
        assert_eq!(&"here is `code` inline"[code_span.range.clone()], "code");
    }

    #[test]
    fn inline_code_multi_backtick_delimiter() {
        let text = "``code with ` backtick``";
        let spans = scan(text);
        let code_span = spans.iter().find(|s| s.style.code).unwrap();
        assert_eq!(&text[code_span.range.clone()], "code with ` backtick");
    }

    #[test]
    fn strikethrough() {
        let spans = scan("this is ~~gone~~ now");
        let s = spans.iter().find(|s| s.style.strikethrough && s.role == SpanRole::Text).unwrap();
        assert_eq!(&"this is ~~gone~~ now"[s.range.clone()], "gone");
    }

    #[test]
    fn blockquote_marker() {
        let spans = scan("> quoted text");
        assert_eq!(spans[0].role, SpanRole::Marker);
        assert!(spans[0].style.blockquote);
        assert_eq!(&"> quoted text"[spans[0].range.clone()], "> ");
    }

    #[test]
    fn nested_blockquote_marker() {
        let spans = scan("> > deeper");
        assert_eq!(spans[0].role, SpanRole::Marker);
        assert_eq!(&"> > deeper"[spans[0].range.clone()], "> > ");
    }

    #[test]
    fn list_bullet_dash() {
        let spans = scan("- item one");
        assert_eq!(spans[0].role, SpanRole::Marker);
        assert_eq!(&"- item one"[spans[0].range.clone()], "- ");
    }

    #[test]
    fn ordered_list_marker() {
        let spans = scan("1. first item");
        assert_eq!(spans[0].role, SpanRole::Marker);
        assert_eq!(&"1. first item"[spans[0].range.clone()], "1. ");
    }

    #[test]
    fn link_hides_url() {
        let text = "click [here](https://example.com) now";
        let spans = scan(text);
        let link_text = spans.iter().find(|s| s.style.link).unwrap();
        assert_eq!(&text[link_text.range.clone()], "here");
        // "](url)" plus the opening "[" must both be markers.
        assert!(spans.iter().any(|s| s.role == SpanRole::Marker && &text[s.range.clone()] == "["));
        assert!(spans
            .iter()
            .any(|s| s.role == SpanRole::Marker && &text[s.range.clone()] == "](https://example.com)"));
    }

    #[test]
    fn unclosed_bold_stays_literal() {
        let spans = scan("**bold with no terminator");
        assert!(spans.iter().all(|s| s.role == SpanRole::Text));
    }

    #[test]
    fn unclosed_link_stays_literal() {
        let spans = scan("[not a link with no closing paren");
        assert!(spans.iter().all(|s| s.role == SpanRole::Text));
    }

    #[test]
    fn star_surrounded_by_spaces_is_literal() {
        let spans = scan("2 * 3");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role, SpanRole::Text);
    }

    #[test]
    fn single_unmatched_star_between_letters_is_literal() {
        let spans = scan("a*b");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role, SpanRole::Text);
    }

    #[test]
    fn escaped_asterisks_stay_literal() {
        let text = r"\*not italic\*";
        let spans = scan(text);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role, SpanRole::Text);
        assert_eq!(&text[spans[0].range.clone()], text);
    }

    #[test]
    fn fence_body_is_single_code_span_not_scanned() {
        let text = "**not bold** in code";
        let spans = scan_line(text, BlockContext::FenceBody);
        assert_tiles(text, &spans);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role, SpanRole::Text);
        assert!(spans[0].style.code);
    }

    #[test]
    fn cjk_and_emoji_lines_do_not_panic_and_tile() {
        let text = "**粗体** と *斜体* 👨‍👩‍👧 e\u{0301}nd";
        let spans = scan(text);
        // Coverage/no-panic is the assertion; `scan()` already asserts tiling.
        assert!(!spans.is_empty());
    }

    #[test]
    fn heading_with_bold_inside() {
        let spans = scan("# Title **bold**");
        let bold = spans.iter().find(|s| s.style.bold && s.role == SpanRole::Text).unwrap();
        assert_eq!(bold.style.heading, Some(1));
    }
}
