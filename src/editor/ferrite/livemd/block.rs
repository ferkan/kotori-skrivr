//! Per-line block context: are we inside a fenced code block?
//!
//! No egui types. Pure logic over `&str` lines, O(n) over the document.
//!
//! Blockquote nesting is deliberately *not* tracked here: unlike fences, a
//! blockquote marker (`>`) is fully determined by the leading bytes of its
//! own line and needs no cross-line state, so `scan.rs` recognizes it
//! directly. `block.rs` owns only the state that genuinely spans lines.

/// Block-level context for a single line, used to decide how `scan.rs`
/// should treat that line's content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockContext {
    /// Ordinary line: run the full inline scanner.
    Normal,
    /// Inside a fenced code block (between the opening and closing fence).
    /// The whole line is literal code text; do not inline-scan it.
    FenceBody,
    /// The opening or closing fence line itself (``` or ~~~, optionally with
    /// a language tag on the opening line).
    FenceDelimiter,
}

/// Returns `true` if `c` is a valid fence character.
fn is_fence_char(c: u8) -> bool {
    c == b'`' || c == b'~'
}

/// Counts leading spaces, capped at 3 (CommonMark allows up to 3 spaces of
/// indentation before a fence is still recognized as a fence rather than an
/// indented code block).
fn leading_spaces(line: &str, cap: usize) -> usize {
    let mut n = 0;
    for b in line.bytes() {
        if b == b' ' && n < cap {
            n += 1;
        } else {
            break;
        }
    }
    n
}

/// If `trimmed` opens a fence, returns `(fence_char, run_length)`.
fn parse_opening_fence(trimmed: &str) -> Option<(u8, usize)> {
    let bytes = trimmed.as_bytes();
    let first = *bytes.first()?;
    if !is_fence_char(first) {
        return None;
    }
    let run_len = bytes.iter().take_while(|&&b| b == first).count();
    if run_len < 3 {
        return None;
    }
    // Backtick fences: the info string after the fence must not itself
    // contain a backtick (CommonMark). Simplification: not enforced here,
    // out of scope for v1 — any trailing text is accepted as a language tag.
    Some((first, run_len))
}

/// Returns `true` if `trimmed` closes a fence opened with `(fence_char, open_len)`.
fn is_closing_fence(trimmed: &str, fence_char: u8, open_len: usize) -> bool {
    let bytes = trimmed.as_bytes();
    let run_len = bytes.iter().take_while(|&&b| b == fence_char).count();
    if run_len < open_len || run_len == 0 {
        return false;
    }
    // Nothing but whitespace may follow the closing fence run.
    trimmed[run_len..].bytes().all(|b| b == b' ' || b == b'\t')
}

/// Computes the `BlockContext` for every line in the document, tracking
/// fenced code blocks. An unclosed fence at EOF simply leaves every
/// remaining line classified as `FenceBody` (no special-casing needed: the
/// loop just never finds a closing delimiter).
pub fn compute_block_contexts(lines: &[&str]) -> Vec<BlockContext> {
    let mut out = Vec::with_capacity(lines.len());
    let mut fence: Option<(u8, usize)> = None;

    for line in lines {
        match fence {
            Some((fence_char, open_len)) => {
                let indent = leading_spaces(line, 3);
                let trimmed = &line[indent..];
                if is_closing_fence(trimmed, fence_char, open_len) {
                    out.push(BlockContext::FenceDelimiter);
                    fence = None;
                } else {
                    out.push(BlockContext::FenceBody);
                }
            }
            None => {
                let indent = leading_spaces(line, 3);
                let trimmed = &line[indent..];
                if let Some((fence_char, open_len)) = parse_opening_fence(trimmed) {
                    out.push(BlockContext::FenceDelimiter);
                    fence = Some((fence_char, open_len));
                } else {
                    out.push(BlockContext::Normal);
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_lines_are_normal() {
        let lines = ["hello", "world"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![BlockContext::Normal, BlockContext::Normal]
        );
    }

    #[test]
    fn simple_backtick_fence() {
        let lines = ["```", "code line", "```"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceDelimiter,
            ]
        );
    }

    #[test]
    fn fence_with_language_tag() {
        let lines = ["```rust", "fn main() {}", "```"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceDelimiter,
            ]
        );
    }

    #[test]
    fn tilde_fence() {
        let lines = ["~~~", "code", "~~~"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceDelimiter,
            ]
        );
    }

    #[test]
    fn unclosed_fence_at_eof() {
        let lines = ["```", "line one", "line two"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceBody,
            ]
        );
    }

    #[test]
    fn indented_fence_up_to_three_spaces() {
        let lines = ["   ```", "  code", "   ```"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceDelimiter,
            ]
        );
    }

    #[test]
    fn four_space_indent_does_not_open_fence() {
        // 4+ spaces would be an indented code block in full CommonMark
        // (out of scope); here it simply fails to match a fence and is
        // classified Normal, which is a safe fallback since we don't claim
        // indented-code-block support.
        let lines = ["    ```", "text"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![BlockContext::Normal, BlockContext::Normal]
        );
    }

    #[test]
    fn closing_fence_must_be_at_least_as_long() {
        let lines = ["````", "code with ``` inside", "````"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceDelimiter,
            ]
        );
    }

    #[test]
    fn shorter_run_does_not_close_longer_fence() {
        let lines = ["````", "```", "still in fence", "````"];
        assert_eq!(
            compute_block_contexts(&lines),
            vec![
                BlockContext::FenceDelimiter,
                BlockContext::FenceBody,
                BlockContext::FenceBody,
                BlockContext::FenceDelimiter,
            ]
        );
    }

    #[test]
    fn empty_document() {
        let lines: [&str; 0] = [];
        assert_eq!(compute_block_contexts(&lines), Vec::<BlockContext>::new());
    }
}
