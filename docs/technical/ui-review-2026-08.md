# UI review remediation — August 2026

Specs for the fixes arising from the combined `design-director-review-mode`
and `emil-design` review of the app shell. Ordered by execution priority.

Each item is self-contained and states its decisions. Anything left open here
is a bug in the spec, not a choice for the implementer.

---

## 0. Icon font (prerequisite for item 5)

Source artwork lives in `assets/icons/editor icons/` (Illustrator exports).
Seventeen of the twenty-five are wired up; the eight with no markdown
equivalent (`align-*` ×4, `underline`, `capitalize`, `columns`, `line-height`)
are deliberately dropped — markdown has no text alignment or underline.

Build pipeline is in `tools/iconfont/` and is reproducible:

```
cd tools/iconfont && npm install
node stage1.mjs        # flatten transforms, normalize stems, expand strokes
python3 build_font.py  # even-odd → nonzero winding, fit to em square, emit TTF
```

Output is committed at `assets/fonts/SkrivrIcons.ttf` (9 KB, 17 glyphs).

Three things the pipeline has to do, each of which was a bug when it did not:

- **Stroke expansion.** Nineteen source icons are `fill:none` strokes. Fonts
  have no strokes, so these are expanded to outlines. `bold` is the one icon
  with no strokes at all and is passed through untouched — running it through
  the outliner introduced a stray sliver contour.
- **Winding conversion.** The outliner emits `fill-rule="evenodd"`. TrueType
  glyphs are nonzero-wound, so every interior filled in solid until
  `pathops.simplify(..., fix_winding=True)` was applied.
- **Stem normalization.** svgo does not flatten the Illustrator
  `scale(4.16667)` group transforms, so a declared `stroke-width` renders at
  4.17× its stated value. Stems are set as a fraction of each icon's own
  larger dimension *divided by* the recovered group scale. All 17 land on an
  identical 5.5% stem; `italic` and `code-block` were outliers at 11.8% and
  7.6% before this.

Glyphs occupy U+E001..U+E011, each fitted to an 860-unit box inside a
1000-unit em with a uniform 1000 advance, so they drop into square button
slots without per-icon nudging.

### Known limitations of the source artwork

These are design issues, not pipeline defects, and are not fixed here:

- `bold` is the only solid-filled glyph in an otherwise hairline-outline set.
  At 16 px it reads noticeably heavier than its neighbours.
- `list-bullet-numbers` and `list-bullet-letters` lose their digits/letters
  below ~18 px and become hard to distinguish from `list-bullet-unordered`.
- `unlink` is visually busy at small sizes.
- `anfang` (initial-letter) and `outline` (hollow serif T) do not map cleanly
  onto any current markdown command; they are in the font but unwired.

---

## 1. Live inline mode is unreachable from the mode switcher

**Severity: broken.** The project's headline feature has no control, and
selecting it makes the control render as though nothing is selected.

`ViewMode` has four variants (`src/config/settings.rs:1700`) but
`ViewModeSegment::show` hard-codes three segments
(`src/ui/view_segment.rs:140-162`). Because `ViewMode::toggle()` cycles
`Rendered → LiveMarkdown` (`settings.rs:1719`), ⌘E lands users in
`LiveMarkdown`, where the selected-indicator loop
(`view_segment.rs:166-194`, breaking at `:191`) never matches and so draws no
pill at all. `LiveMarkdown` is also offered as a startup default via
`ViewMode::all()` (`settings.rs:1769`).

### Decisions

- Drive the segments from `ViewMode::all()` rather than a literal array.
- Four segments at `SEGMENT_WIDTH` 26 = 104 px, inside the 400 px reserved
  button area (`src/app/title_bar.rs:126`). Split stays; nothing is demoted.
- Order follows `ViewMode::all()`: Raw, Rendered, Split, LiveMarkdown.
- Give `LiveMarkdown` its own glyph. `ViewMode::icon()` (`settings.rs:1745`)
  currently reuses `EYE` for it with a comment saying new icon assets were out
  of scope; that is no longer true, but the new font has no view-mode glyph
  either, so keep Phosphor here and use `PENCIL_LINE` for `LiveMarkdown` to
  distinguish it from `Rendered`'s `EYE`.
- If a mode is somehow not in `all()`, draw no pill but still render the
  segments — never panic.

### Work

- `src/ui/view_segment.rs:140-162` — replace the literal tuple array with a
  loop over `ViewMode::all()`, deriving icon from `mode.icon()`, tooltip from
  `mode.label()` + `mode.description()`, and enablement from the existing
  file-type rule (Split disabled for non-markdown structured files).
- `src/ui/view_segment.rs:166-194` — indicator loop keyed on the same slice.
- `src/config/settings.rs:1745` — `PENCIL_LINE` for `LiveMarkdown`; drop the
  now-stale comment.
- Widen the control from 3 to 4 segments where the width is computed.
- `show_two_mode` (`view_segment.rs:256`) is a separate two-mode path for
  non-markdown files; leave its behaviour alone.

### Acceptance

Cycling ⌘E through all four modes always leaves exactly one pill drawn, and
launching with `LiveMarkdown` as the saved default shows it selected.

---

## 2. File tree never indicates which file is open

**Severity: broken.** In a 40-file workspace the sidebar cannot answer
"where am I?".

`src/ui/file_tree.rs:240` computes `_selected_bg` and never uses it. The row
paint path (`:272-279`) draws only a hover background. Neither
`FileTreePanel::show` (`:125`) nor `render_tree_node` (`:216`) receives the
active file.

### Decisions

- Thread `Option<&Path>` for the active file through both functions.
- Selection and hover are visually distinct, because a selected row can also
  be hovered: selection is `_selected_bg` fill **plus** a 2 px accent bar
  along the row's left edge; hover stays fill-only.
- Paint order: selection fill, then hover fill, then the accent bar, then
  text. The accent bar must survive hover.
- Compare canonical paths, not display strings.

### Work

- `src/ui/file_tree.rs:125` — add `active_file: Option<&Path>` to `show`.
- `src/ui/file_tree.rs:216` — same parameter on `render_tree_node`, passed
  down recursively.
- `src/ui/file_tree.rs:272-279` — implement the paint order above; rename
  `_selected_bg` to `selected_bg`.
- `src/app/mod.rs:1908` — pass `self.state.active_tab().and_then(|t| t.path.as_deref())`.

### Acceptance

Opening a file marks exactly one tree row; hovering a different row leaves the
selected row still identifiable.

---

## 3. Dark-theme contrast and accessible names

**Severity: broken.** Two independent defects, both shipping.

### 3a. White-on-accent is 2.2:1

Default accent is `(100, 180, 255)` (`src/theme/accent.rs:6`); against
`Color32::WHITE` that is 2.2:1, below the 3:1 floor even for large text.

- `src/theme/dark.rs:98` — `widgets.active.fg_stroke` is `WHITE` over
  `active.bg_fill = colors.ui.accent` (`:95`). Every pressed button label.
- `src/ui/view_segment.rs:88`, `:105-109` — selected segment fill is the raw
  accent with a `(255,255,255)` glyph at 11 px (`:230`).

`src/theme/light.rs:95-98` already hit this and documents why `WHITE` was
rejected there. Dark never got the same treatment.

**Decision:** add `accent::on_accent(accent: Color32) -> Color32` next to
`accent_hover`, returning near-black (`(20,20,20)`) when the accent's relative
luminance exceeds 0.4 and near-white (`(250,250,250)`) otherwise. Use WCAG
relative luminance, not a naive average. Fix both call sites through it. This
must stay correct for user-chosen accents, which are unconstrained at
`src/ui/welcome.rs:141`.

### 3b. Window controls are nameless to screen readers

`accesskit` is enabled (`Cargo.toml:36`) but `widget_info` appears **zero
times** in `src/`. The pattern `Button::new(RichText::new(" "))` plus a manual
`painter().text()` gives these buttons a literal single space as their
accessible name:

- `src/app/title_bar.rs:188` close, `:223` maximize, `:322` minimize,
  `:350` fullscreen
- `src/ui/view_segment.rs:445` `TitleBarButton::show` (settings, zen mode),
  `:504` `show_two_mode` (auto-save)
- `src/ui/format_toolbar.rs:492` `toolbar_icon_button` (TOC)

**Decision:** pass the glyph as the button's own text — the pattern already
used correctly at `format_toolbar.rs:314` — and attach
`response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label))`
using the same string as the tooltip. Remove the manual `painter().text()`
wherever the glyph now renders itself. Tooltip strings stay as they are; this
adds a name, it does not replace the tooltip.

### 3c. Focus is invisible

No `has_focus()` call exists in `src/ui/`. Controls built on raw
`allocate_rect`/`interact` take `Sense::click()` and render identically
focused and unfocused: tab bar and close buttons
(`src/app/central_panel.rs:356`, `:437`, `:496`), view segment
(`view_segment.rs:207`, `:362`), file tree rows (`file_tree.rs:272`), side
panel strip (`format_toolbar.rs:574`).

**Decision:** after each interact, `if response.has_focus()` stroke the rect
with `ui.visuals().selection.stroke` at the same corner radius as the fill.
No layout change.

### 3d. Hit targets

`format_button` is 22×20 (`format_toolbar.rs:436`), segments are 26×20
(`view_segment.rs:19-20`), tree rows are 20 px (`file_tree.rs:268`). WCAG 2.2
floor is 24×24.

**Decision:** raise `format_button` to 24×24 and `SEGMENT_HEIGHT` to 24. Tree
rows stay at 20 — raising them changes information density materially and is a
design-director call, not a bug fix. Record it as deferred.

### 3e. Press state

`format_button` has hover and active-toggle states but no pressed state, and
`.frame(false)` disables egui's own feedback, so a click gives no confirmation
until the document changes.

**Decision:** add an `is_pointer_button_down_on()` branch painting a fill one
step darker than hover. No animation — this is a 100+/day interaction and the
global `animation_time = 0.0` (`src/app/mod.rs:230`) is correct and stays.

---

## 4. Dead per-frame formatting scan

**Severity: latency.** `is_inside_code_block`
(`src/markdown/formatting.rs:799-814`) iterates every line from byte 0 to the
cursor, uncached, with no early exit. It is reached twice per frame:

- `src/app/mod.rs:1571-1577` for the ribbon — **pure waste**. `Ribbon::show`
  (`src/ui/ribbon.rs:164`) already takes the parameter as `_formatting_state`
  and never reads it; the format buttons moved out per `ribbon.rs:506`.
- `src/app/central_panel.rs:822-832` for the format toolbar — needed.

On a 200 KB document with the cursor at the end, that is two full scans per
frame while typing.

### Decisions

- Delete the `mod.rs:1571-1577` computation and drop the `_formatting_state`
  parameter from `Ribbon::show` entirely, updating the call at `:1600`.
- Cache the remaining result on `Tab`, keyed on `(content_version, cursor_line)`.
  `Tab` already has `content_version: u64` (`src/state.rs:1286`) and
  `source_epoch: u64` (`:1291`) — include `source_epoch` in the key so
  external content changes invalidate correctly.
- Cursor movement *within* a line does not change block-level state, so column
  is deliberately not part of the key. Inline state (bold/italic/code) is
  column-dependent, so cache only the block-level portion and keep the inline
  detection uncached — it is bounded by the current line.

### Acceptance

Typing at the end of a large document does not re-scan the document. Add a
test asserting the cache is reused when only the column changes.

---

## 5. Theme routing and icon swap

**Severity: cohesion.**

`src/ui/format_toolbar.rs` defines eleven literal RGB pairs branched on a raw
`is_dark: bool` (`:43-59`, `:394-416`, `:533-549`), none from `ThemeColors`.
The visible cost: a toggled-on Bold button is a fixed `(70,90,120)` blue while
every other "on" affordance uses the user's accent via `apply_user_accent`
(`src/theme/mod.rs:82`). `src/ui/view_segment.rs:439` adds a third colour —
`(60,90,60)`, "green-ish for active" — in adjacent chrome.

### Decisions

Replace `is_dark: bool` with `&ThemeColors` on `FormatToolbar::show` (`:33`),
`format_button` (`:385`), `toolbar_icon_button` (`:465`) and
`side_panel_toggle_strip` (`:528`). Mapping:

| role | field |
|---|---|
| bar background | `colors.base.background_secondary` |
| separator | `colors.base.border` |
| chevron | `colors.text.muted` |
| active fill | `colors.base.selected` |
| disabled glyph | `colors.text.disabled` |
| active glyph | `accent::on_accent(colors.base.selected)` (from item 3a) |

`on_accent` takes the color the glyph actually sits **on**. The active fill is
`base.selected`, not the raw `ui.accent`, so it must be measured against that —
they are different colors and give different answers.

`ThemeColors` is built at `src/app/mod.rs:1548` but not currently passed to
`render_central_panel` (`:2235`); thread it through
`src/app/central_panel.rs:240`.

Then swap the toolbar's Phosphor glyphs for the new font: register
`SkrivrIcons.ttf` in `src/fonts.rs` alongside `register_phosphor_icon_font`
(`:2204`), add `src/ui/skrivr_icons.rs` mirroring `phosphor_icons.rs`, and map
bold, italic, code-block, quote, link, unlink and the three list variants.
Icons not covered by the new font keep their Phosphor glyphs — do not leave a
half-swapped bar with two icon systems at different weights; if a needed glyph
is missing, keep Phosphor for that whole button group and note it.

### Also in this file

- `:1-8` module docs say the bar is "at the bottom of the raw editor"; it has
  rendered at the top since `173710e`.
- `:24-25` orphaned doc comment whose constant was deleted; the collapsed
  height `18.0` now sits as a magic number at `central_panel.rs:851`,
  decoupled from `TOOLBAR_HEIGHT_EXPANDED` (`:22`). Reunite them.
- `central_panel.rs:1094` passes `has_editor: true` as a literal, making every
  disabled branch (`:400-404`, `:478-482`) unreachable. Wire it to real state
  or delete the parameter — do not leave implied coverage that does not exist.

---

## 6. Deferred list

**Status as of 2026-08-09.** This section is the fork's running design backlog.
Items are struck through as they land; the code is the authority, so verify
before acting on anything still listed as open — this list has over-reported
before.

### Resolved

- [x] **Chrome height.** Was ~158 px of permanent chrome above the first line of
  text (title bar 35, ribbon 36, tab bar 32, format toolbar 32, status bar 24).
  The ribbon and format toolbar — two icon strips 60 px apart, split for
  historical rather than design reasons — are now **one merged bar**
  (`app/mod.rs`, the `"toolbar"` panel at `ui::TOOLBAR_HEIGHT` 32), and the
  title bar is down to 28 (`title_bar.rs:81`).
- [x] **Product name.** `welcome.rs` reads `branding::APP_NAME`, no longer the
  literal "Ferrite".
- [x] **`MARKER_ALPHA_SCALE`** raised 0.35 → 0.55 (`livemd/style.rs:25`).
- [x] **Untranslated strings.** The ~30 listed here are down to zero. Keys were
  added to `locales/en.yaml` only — `src/main.rs:30` sets `fallback = "en"`, so
  the other nine locales fall back cleanly, and adding English copies to them
  would only disguise what still needs translating.
- [x] **Mixed icon systems in the tab strip.** Image, PDF, loading and error
  tabs used Unicode emoji while special tabs used Phosphor — different baseline
  and optical weight in the same strip. All four are now Phosphor (`IMAGE`,
  `FILE_PDF`, `HOURGLASS`, `WARNING`).

  This forced a split in `Tab::title()`: it now composes
  `title_icon()` + `plain_title()`, and every surface that cannot render a
  private-use-area glyph — the OS window title, session/recovery metadata —
  takes `plain_title()` and gets a clean label instead of tofu.
- [x] **Unsaved-changes indicator.** The trailing `*` is gone from
  `Tab::title()`, which is now pure identity. A filled dot occupies the
  close-button slot and yields to the close `×` on hover. Screen readers get
  the state from the tab's accessible name, which the `*` was previously the
  only carrier of.
- [x] **Nothing truncated.** Tab widths were unbounded — one long filename ate a
  whole row of a wrapping tab strip. Now capped at `MAX_TAB_WIDTH` 220 px with
  an ellipsised galley, and a tooltip *only* when the name actually truncated.
  File-tree names are bounded the same way, against the row width minus the
  git-status badge.
- [x] **Disabled segments were illegible** — measured **1.3:1** dark and
  **1.4:1** light, not the 1.8:1 originally recorded. Now derived, not
  hardcoded: mute the enabled text halfway toward the background, then let
  `accent::readable_on(..., 3.0)` pull it back to the floor. Lands at 3.25:1
  dark / 3.02:1 light and stays correct in both themes from one expression,
  which is what the hardcoded pair failed to do. Guarded by
  `theme::contrast_tests::disabled_view_segment_text_meets_contrast_floor`.

  The disabled *reason* was reachable only by hovering. Every segment now
  carries an accessible name, disabled ones with the reason appended.

### Still open

- **File-tree row height is 22 px** against a WCAG 2.2 floor of 24 (item 3d
  deferred this at 20 px; it has since gone to 22). Raising it changes sidebar
  information density materially — a design-director call, not a bug fix.
*(Nothing else. The two accessible-name strings introduced by this pass —
the disabled-segment reason and the tab strip's "modified" suffix — went
straight to `view_mode.unavailable_reason` and `tab.modified_suffix` rather
than being logged here as debt.)*

---

## 7. Literata as the default editor body font

Inter is a UI grotesque — fine for chrome, thin for a page of prose read for
minutes at a time. Literata (`src/fonts.rs`, `src/config/settings.rs`) is now
the **default** `EditorFont` for the editing surface. Inter stays the
`FontFamily::Proportional` UI font for every panel, dialog and toolbar;
JetBrains Mono stays for all code. Only the body-text font changed.

The four static cuts embedded (`assets/fonts/Literata-*.ttf`) are pinned from
the variable source at `opsz=16` (the default body size uses the 16pt optical
master, not the variable font's 12pt default) and `wght` 400/600. **Bold uses
the 600-weight SemiBold cut, not 700** — a 700-weight serif shouts at heading
sizes and breaks up inline `**bold**` paragraph texture; 600 stays legible
without doing that. See `tools/bodyfont/README.md` for how to regenerate the
files from upstream Literata.
