---
name: livemd
description: Architecture contract for the live inline (Typora-style) WYSIWYG markdown mode. Read this before touching anything under src/editor/ferrite/livemd/, the editor render loop, or cursor/mouse column mapping. Defines the span model, the source-to-display column mapping rule, and the reveal policy.
---

# Live inline markdown (`livemd`) — architecture contract

The feature: **one continuous editing surface** where markdown renders styled as
you type, and syntax markers are hidden except on the line the cursor is on.

```
cursor NOT on line:   Some bold text here.       <- styled, ** hidden
cursor ON line:       Some **bold** text here.   <- markers revealed
```

This is a **new** mode. It is not the upstream `ViewMode::Rendered`, which is
block-level click-to-edit (click a block, it swaps to a raw `TextEdit`). Both
coexist; do not refactor upstream's rendered mode into this.

## The one hard problem

When markers are hidden, **the text on screen is not the text in the buffer**.
Every existing coordinate path assumes they are identical. That assumption is
the source of essentially every bug this feature can have.

So: **a column is either a source column or a display column, never "a column".**
Name every variable accordingly (`src_col`, `disp_col`). Mixing them produces
cursor drift that is very hard to trace back — upstream already shipped one
such bug in their block-based mode ("click-to-edit cursor drift on mixed-format
lines", still open in their v0.3.1 roadmap). We avoid it by construction.

The buffer, undo history, save, and search **always** operate in source columns.
Only rendering and hit-testing touch display columns.

## Module layout

New module `src/editor/ferrite/livemd/`:

| File | Responsibility | Depends on egui? |
|------|----------------|------------------|
| `block.rs` | Per-line block context (inside fenced code? blockquote depth?) | No |
| `scan.rs` | Inline scanner: source line → styled spans | No |
| `map.rs` | `LineMap`: source column ↔ display column | No |
| `style.rs` | `InlineStyle` → egui `TextFormat` | Yes |
| `mod.rs` | Re-exports | — |

`block.rs`, `scan.rs`, `map.rs` are **pure logic with no egui types**. That is
deliberate: they are unit-testable without a GUI context, and that is where the
correctness risk lives. Keep egui out of them.

## Data model

```rust
/// Whether a span is content the user reads, or syntax that can be hidden.
pub enum SpanRole { Text, Marker }

/// Byte range within the SOURCE line, plus how to draw it.
pub struct StyleSpan {
    pub range: std::ops::Range<usize>, // byte offsets into the source line
    pub style: InlineStyle,
    pub role: SpanRole,
}

pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,            // inline `code` or inside a fence
    pub link: bool,
    pub heading: Option<u8>,   // 1..=6, scales font size
    pub blockquote: bool,
}
```

Ranges are **byte offsets, not char offsets**. This codebase handles CJK, IME
and complex scripts; char-indexing a `&str` is a panic waiting to happen. Use
`str::get(range)` and grapheme-aware helpers from `src/editor/ferrite/grapheme.rs`.

## Reveal policy

A line renders **revealed** (markers visible, still styled) when:

- the primary cursor is on that line, **or**
- any selection intersects that line.

Otherwise it renders **hidden** (Marker spans omitted from the display string).

Consequence: **the galley cache key must include the reveal bit.** The same line
text produces two different galleys depending on reveal state. Forgetting this
is a silent stale-render bug — the line will simply not update when the cursor
arrives. `LineCache` keys on a content hash (`line_cache.rs`, `CacheKey::new*`),
so the reveal flag has to be hashed in alongside content, font and color.

## Where this plugs into the existing editor

Anchors verified against our import commit:

- **Render loop:** `src/editor/ferrite/editor.rs:1656-1760`. Not `view.rs` —
  `view.rs` is viewport/scroll bookkeeping only and makes no `LineCache` calls.
  There are five paths, gated on `effective_wrap_enabled` and `use_syntax`
  (wrapped+syntax, wrapped plain, unwrapped+syntax, unwrapped complex-script,
  unwrapped Latin fallback). Live mode must be handled in each, or explicitly
  excluded from it.
- **Segment production:** `FerriteEditor::highlight_line(&self, line_content, language)
  -> Vec<HighlightedSegment>` at `editor.rs:3021`. This is the natural hook —
  livemd produces a richer segment type alongside it, not instead of it.
- **`HighlightedSegment`** (`line_cache.rs:93`) carries only `text` + `color`.
  It cannot express bold/italic/size. Add a new richer segment type; **do not
  repurpose or break `HighlightedSegment`** — the syntect path still uses it.
- **Cursor x-position:** `rendering/cursor.rs` — unwrapped uses
  `shaping::shaped_column_to_x` / galley width (line 106+); wrapped uses
  `galley.pos_from_cursor` (line 200).
- **Mouse → column:** `mouse.rs:14` `calculate_column_from_pos` — wrapped uses
  `galley.cursor_from_pos` (line 40); unwrapped uses `shaping::shaped_x_to_column`
  (line 51).
- **Main state:** `pub struct FerriteEditor` at `editor.rs:80`.
- **No fence tracking exists** anywhere under `src/editor/ferrite/`. Block
  context is genuinely new work — `block.rs` owns it.

### The mapping rule

Both cursor rendering and mouse hit-testing must funnel through `LineMap`:

- Drawing the cursor: `src_col` → `LineMap::to_display(src_col)` → then the
  existing x-position code, run against the **display** string.
- Handling a click: existing code yields `disp_col` against the display string →
  `LineMap::to_source(disp_col)` → then everything downstream is source columns.

On a **revealed** line the map is the identity. That makes revealed lines a
useful control when debugging drift: if a bug reproduces on a revealed line, it
is not a mapping bug.

`to_source` must be total — a click landing "inside" a hidden marker has to
resolve to a defined source column. Rule: **snap to the start of the hidden run**
(so clicking where `**` used to be puts the caret before the bold text, and
arrowing left from there reveals the line). Never return an index that can land
mid-UTF-8-sequence.

### Which round-trip is required (resolved)

`to_source` is **deliberately not invertible from the source side.** A hidden
marker run occupies real source bytes but zero display width, so multiple source
columns collapse onto one display column. For `some **bold** text`, both source
column 5 (start of `**`) and source column 7 (start of `bold`) map to display
column 5; `to_source(5)` returns 5 per the snap rule, so
`to_source(to_display(7)) != 7`. This is inherent to collapsing a range to a
point, not a defect, and it is **not** to be "fixed."

The invariant that *is* required, and that must be tested, is the **display-side**
round-trip:

```
to_display(to_source(d)) == d    for every display column d
```

This is the direction the user can see: click at display column `d`, and the
caret must be drawn back at `d`. Plus **`to_source` must be monotonic
non-decreasing**, or a rightward arrow key could move the caret leftward in the
buffer.

Rationale for the snap direction: the caret lands *outside* the emphasis, so the
line then reveals as `some |**bold** text` and typing extends unemphasised text.
That matches Typora/Obsidian behaviour and keeps reveal coherent.

## Fonts — already solved

Bold and italic **do** render; the families are registered in `src/fonts.rs`:

- `FONT_INTER`, `FONT_INTER_BOLD`, `FONT_INTER_ITALIC`, `FONT_INTER_BOLD_ITALIC`
  (constants at `fonts.rs:917-921`)
- `FONT_JETBRAINS*` equivalents (`fonts.rs:930-937`)

`style.rs` maps `InlineStyle` → `FontFamily::Name(...)` by picking the variant
matching the editor's current `EditorFont` (`config/settings.rs:1532`). Inline
`code` always uses the JetBrains family regardless of editor font. Headings
scale `FontId::size`; do not try to emulate weight with color.

## Scope boundaries for v1

**In:** headings, bold, italic, bold-italic, inline code, strikethrough,
blockquote markers, list bullets/numbers, fenced code blocks (dimmed fence
lines, mono body), link text with hidden URL.

**Out (deliberately, do not build these yet):** tables, images rendered inline,
footnotes, math, wikilinks, embedded HTML. They need block-level layout that
this per-line model cannot express, and forcing them in will compromise the
line model. Leave them rendering as raw styled text.

## Testing expectation

`scan.rs` and `map.rs` ship with unit tests. Non-negotiable cases:

- Round-trip: for every source column, `to_source(to_display(c)) == c`.
- CJK / emoji / combining-mark lines — no panics, no byte-boundary slicing.
- Unclosed markers (`**bold` with no terminator) render as literal text.
- Nested emphasis (`***both***`, `**bold with `code` inside**`).
- A `*` that is not emphasis (`2 * 3`, `a*b`) stays literal.
- Escaped markers (`\*not italic\*`).
- Fence body must not be inline-scanned — `**` inside a code block is literal.
