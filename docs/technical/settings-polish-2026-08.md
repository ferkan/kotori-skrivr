# Settings & chrome polish — August 2026

Follow-up to `ui-review-2026-08.md`, from a screenshot pass over the settings
pages after the layout system landed. Ordered by execution priority.

Each item states its decision. Anything left open here is a bug in the plan,
not a choice for the implementer.

---

## 0. Chrome typography — the brief

> "The typography of the settings pages should match that of the editor views.
> It should be pleasant to read."

**Interpretation, and the one judgement call in this document.** The editor is
Literata at 16 px / 1.4 leading on a warm ground, and it reads well. Settings
is Inter at 13 px with no explicit leading — noticeably tighter and smaller.

Two readings:

1. *Same comfort* — raise chrome sizes and give chrome real leading, keep Inter.
2. *Same typeface* — set settings in Literata too.

**Taken: (1).** The sans/serif split is the app/document boundary — serif is
"your words", sans is "the app's words" — and it is the clearest structural
signal a writer has when glancing up mid-thought. Setting a preferences form in
a reading serif also tends to read as a document that happens to have controls
in it.

What was actually wrong is that chrome was *small and tight*, not that it was
sans. So:

| role | before | after |
|---|---|---|
| `chrome::TITLE` (page title) | 18 | 18 |
| `chrome::SECTION` (new) | — | **15, bold** |
| `chrome::BODY` (labels, rows) | 13 | **14** |
| `chrome::SMALL` (hints, status) | 12 | **12.5** |
| `chrome::LINE_HEIGHT` (new) | none | **1.45** |

Chrome gets explicit leading for the first time; previously it inherited each
font's native metric, which is the same defect the editor had.

If the serif reading is preferred, it is a one-line change to
`get_base_font_family` for the settings surface — say so and it is done.

---

## 1. Warning text is unreadable in the light theme

**Severity: broken.** `UiColors::light().warning` is `(255, 193, 7)`, which on
the `#FBF9F5` page measures **1.55:1**. The code-execution security notice is
effectively invisible.

The dark theme's `(255, 210, 50)` measures 11.90:1 and is fine — this is a
light-only defect, which is why it survived: the amber was picked for dark and
never re-checked against the warm page.

**Decision:** darken the light warning to `(145, 110, 4)` — **4.50:1**, hue
preserved. Dark unchanged. Add it to `theme/contrast_tests.rs` so it cannot
regress.

---

## 2. Sidebar (screenshot 9)

- Category buttons are centre-aligned; they must be **left-aligned**, with the
  icon and label sharing a consistent left edge. Indented relative to the
  "Settings" heading is fine and preferred.
- The "Settings" heading sits flush against the panel edge. It needs left
  padding matching the category indent's outer edge.

Anchors: `src/ui/settings.rs:257-289`.

**Decision:** one `SIDEBAR_PADDING_X` constant, applied to both the heading and
the button column, so they cannot drift apart. Buttons get
`Layout::left_to_right` content alignment rather than the default centring.

---

## 3. Dropdown columns are ragged (screenshot 10)

In "Additional Scripts" each dropdown begins immediately after its label, and
the labels vary from "Thai" to "Southeast Asian" — so the controls form a
staircase.

**Decision:** a fixed label column. `LABEL_COLUMN_WIDTH` (140 px), label
left-aligned within it, control starting at a constant x. Every settings row
with a label + control uses it, so all controls in a page share one left edge.

Banding on this list stays — it is a long homogeneous list, which is what
banding is for.

---

## 4. Checkbox grids must not be banded (screenshots 12, 13)

The two-column checkbox grids in the Editor page are banded. Banding a
two-column grid tints *pairs of unrelated settings* and reads as a highlight on
whichever happens to land on an odd row.

**Decision:** remove banding from `editor_toggles_grid` and the code-folding
sub-grid. Banding applies to lists where a row is one thing; grids stay
uniform.

**Also:** "Show Minimap" sits alone on row 2 with an empty right cell, because
the grid is filled pair-by-pair from a list of odd length. Items must flow to
fill both columns with no gap, or — where a control is genuinely full-width —
that must be deliberate rather than a side effect of the count.

Anchors: `src/ui/settings.rs:1300-1340`.

---

## 5. Dropdowns need internal padding (screenshot 14)

Text sits flush against the left edge of the combo box.

**Decision:** `ui.spacing_mut().button_padding` raised for combo boxes in
settings, so the selected value has breathing room on both sides. Applied
centrally, not per control.

---

## 6. Section headings are not distinct enough (screenshot 15)

"File" / "Navigation" / "View" in the keyboard list read at the same weight as
the rows beneath them.

**Decision:** `section_heading` moves to `chrome::SECTION` (15, bold). The
previous "weight and space alone at body size" call was too subtle in a dense
list; size *and* weight *and* space.

---

## 7. Toolbar icons are too small and too tight (screenshot 16)

Phosphor glyphs render at 11 px and Skrivr at 13 px in a 32 px bar, with 2 px
item spacing.

**Decision:** Phosphor → **15**, Skrivr → **16** (the Skrivr set is fitted to a
uniform em square and reads slightly smaller at the same nominal size), item
spacing 2 → **4**, separator margins widened to match. Buttons keep their
24×24 hit target; only the glyph and the spacing grow.

---

## 8. Sequencing

1. §1 warning contrast — a security notice nobody can read
2. §0 chrome scale — the brief, and it changes every measurement below it
3. §6 section headings — depends on §0
4. §2 sidebar alignment
5. §4 grid banding + the empty cell
6. §3 label column
7. §5 dropdown padding
8. §7 toolbar icons

§0 first among the visual items because raising `chrome::BODY` changes row
heights everywhere, and doing the alignment work against the old metrics would
mean redoing it.
