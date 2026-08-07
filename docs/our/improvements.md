# Fork improvement backlog

Findings from surveying the imported Ferrite v0.3.0 base. Ranked by value.
Anchors are against our import commit (`dc7fb06`).

Status key: **[verified]** = coordinator read the code and confirmed the
mechanism. **[reported]** = found by survey, not yet independently confirmed.

---

## 1. Wrapped-scroll stutter — root cause found [verified]

Upstream's ROADMAP lists "Wrapped line scroll stuttering" as an open issue with
cause unknown ("Likely related to per-line galley layout cost or height cache
granularity"). It is neither. The cause is the height-cache rebuild.

**Mechanism.** `ViewState::set_line_wrap_info` (`src/editor/ferrite/view.rs:838-842`):
when a line is rendered for the first time, `wrap_info` is grown and
`dirty_from_line` is set to the *old* vector length. Then
`rebuild_height_cache` (`view.rs:857`) loops:

```rust
for i in rebuild_from..total_lines {   // view.rs:883
    cum_h += self.get_line_height(i);
    ...
}
```

It runs to `total_lines`, not to the last line actually rendered. So every frame
that scrolls into not-yet-rendered territory re-walks the entire remainder of
the document. On a 50k-line file scrolled to line 1000, that is ~49,000
iterations per frame — and almost all of them just add the *default* height,
because those lines have no real wrap info yet.

The doc comment at `view.rs:851-853` claims this is incremental and O(total -
dirty_from). That is true of the prefix reuse, and it is exactly why the real
problem went unnoticed: the optimisation that was added is real, but it only
bounded one end of the loop.

**Fix.** The suffix beyond the last-measured line is
`(total_lines - measured_len) * default_line_height` — an O(1) closed form. Walk
only up to `wrap_info.len()`, then add the closed-form tail. Keep
`total_content_height` exact so the scrollbar does not jump.

**Effort:** M. **Risk:** medium — scrollbar geometry and `scroll_to_line` both
read these arrays, so this needs a test on cumulative-height correctness.

---

## 1b. Crash-recovery can inject content into the wrong file [verified]

Found by repairing the test target (see §5) — the test asserting this was
already written and simply could not run.

**Failing tests:** `state::tests::test_recovery_identity_path_mismatch_rejected`
and `test_recovery_identity_original_bleeding_repro_rejected`.

**Symptom.** An *untitled* tab's recovery snapshot from a previous session can be
applied to a *different, path-backed* file that happens to reuse the same tab id.
The user opens a real document and finds unrelated content from a scratch buffer
in it.

**Mechanism.** `AppState::try_apply_recovery` (`src/state.rs:5279`) has a
correct path-equality guard at `state.rs:5299` — but an escape hatch above it
runs first (`state.rs:5283`):

```rust
let is_legacy = recovered.path.is_none() && recovered.original_content_hash.is_none();
if is_legacy { return Some(ResolvedContent::Recovered(...)); }  // skips the guard
```

The intent is "old recovery files predate identity fields, so trust them." But
`path: None` is **also** exactly what a legitimate untitled-tab recovery looks
like. So every untitled recovery is misclassified as legacy and bypasses the
identity check the surrounding code carefully implements.

**Why the obvious fix does not work.** `RecoveryContent` has a `schema_version`
field (`config/session.rs:343`) that looks like the right discriminator — but
`RECOVERY_CONTENT_SCHEMA_VERSION` is `1` (`session.rs:294`) *and* files missing
the field also deserialize to `1` (`session.rs:362`). Legacy and current files
are therefore indistinguishable by version. The marker exists but was never
given a distinguishing value.

**Fix.** Bump `RECOVERY_CONTENT_SCHEMA_VERSION` to `2` (newly written files get
2; files lacking the field still default to 1), then make the check
`is_legacy = recovered.schema_version < 2`. Untitled recoveries written by
current code then carry version 2 and correctly fall through to the path guard.

**Effort:** S. **Risk:** low, but it *is* a real behaviour change to recovery
semantics and it touches user data, so it wants a deliberate decision rather
than being folded into unrelated work. Not yet applied.

---

## 5. The test suite could not compile at all [verified — FIXED]

`cargo build` passed, so this went unnoticed: `cargo check --all-targets` failed
with 42 errors, entirely in `#[cfg(test)]` modules across 5 files (missing or
renamed imports in `history.rs`, `mermaid/mod.rs`, `flowchart/render/edges.rs`,
`code_execution.rs`, plus a test calling a `Task::to_markdown` that was never
implemented).

**This inverts the "no tests" picture below.** The project has **1,644 passing
tests** — a substantial suite. They simply had not been runnable, which also
meant **5 genuine failures were hidden**, including the recovery bug in §1b.

Fixed in commit `1f53046`: imports repaired, `Task::to_markdown` implemented as
the inverse of the existing `from_markdown` that a test already specified.

Remaining known failures (all pre-existing, none yet fixed):

| Test | Area |
|------|------|
| `state::tests::test_recovery_identity_path_mismatch_rejected` | **data integrity — see §1b** |
| `state::tests::test_recovery_identity_original_bleeding_repro_rejected` | **data integrity — see §1b** |
| `vcs::git::tests::test_git_service_untracked_file` | git status reports Clean for untracked |
| `markdown::mermaid::tests::test_subgraph_title_width_expansion` | diagram layout |
| `markdown::video_embed::tests::document_parses_bare_youtube_url_paragraph` | parsing |

**Recommendation:** add `cargo check --all-targets` to CI, not just
`cargo build`. That single gap is what allowed the suite to rot unnoticed.

---

## 2. Thin coverage on the highest-risk files [reported]

Note §5 first: coverage overall is much better than this item alone suggests.
Still, zero `#[cfg(test)]` modules in:

| File | LOC | What it does |
|------|-----|--------------|
| `src/app/central_panel.rs` | 3466 | input dispatch |
| `src/app/mod.rs` | 2870 | app lifecycle |
| `src/app/file_ops.rs` | 2155 | **open / save / reload — disk I/O** |
| `src/editor/ferrite/vim.rs` | 842 | hand-rolled modal state machine |

`file_ops.rs` is the one that matters most: it is the code that can lose a
user's work, and it has no automated coverage at all. `vim.rs` is a hand-rolled
state machine, the classic shape for silent regressions.

**Effort:** M for smoke tests on the file_ops save/load round-trip; L to cover
properly. Recommend the former first.

---

## 3. `show_rendered_editor` is an 822-line function [reported]

`src/markdown/editor.rs:1004-1826`, inside a 7091-line file.

Good news: the rest of the file is *not* one monolith — it is ~70 free functions
with seams already visible by name. Extractions that need almost no logic change:

- Block renderers (heading/paragraph/blockquote/callout/list/table/code/mermaid) — lines 2893-5277
- Inline/link/image rendering — lines 4231-5991
- **Source-mutation helpers — lines 6026-6324.** Do this one first: they are
  pure, take no `Ui`, and are therefore directly unit-testable. Best
  value-per-risk in the whole file.
- Formatted-edit session state machine — lines 497-2710

**Effort:** L in aggregate, but each bullet is independently M.

---

## 4. Diffuse dead scaffolding [reported]

93 `#[allow(dead_code)]` annotations across 39 files (`ui/ribbon.rs`,
`markdown/mermaid/flowchart/layout/*`, `editor/ferrite/{buffer,cursor,history,search}.rs`).
Plus the 8 dead-code warnings the baseline build emits. Accreted scaffolding
rather than one hotspot — worth a single sweep to delete or justify each.

**Effort:** S per file, M in aggregate. Low priority.

---

## Negative findings — do not re-investigate

These were checked and are **not** problems. Recorded so nobody spends tokens
re-deriving them.

- **Panic risk on the render/input path is low.** 706 `unwrap()/expect()/panic!`
  hits across `src/`, but the overwhelming majority are inside `#[cfg(test)]`
  modules. `editor/ferrite/editor.rs` has only 3 non-test unwraps (lines 1672,
  1719, 2513) and all are provably guarded by a preceding check.
- **LSP is cleanly feature-gated, not half-wired.** `Cargo.toml:13` excludes
  `lsp` from default features and `src/main.rs:40-44` swaps in `lsp_stub.rs`.
  The alarming-looking `spec.unwrap()` at `src/app/file_ops.rs:744` is
  unreachable when `None` — `sync_active_doc_to_lsp` returns early at
  file_ops.rs:715-725.
- **No regex is compiled inside the per-line render loop.** Checked; that
  common egui performance smell is absent here.
