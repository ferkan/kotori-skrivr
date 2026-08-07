# Kotori Skrivr — notes for coding agents

A fast native Markdown editor in Rust + egui. Forked from
[Ferrite](https://github.com/OlaProeis/Ferrite) v0.3.0 by OlaProeis (MIT).
Git history starts fresh at the initial Kotori Skrivr commit; upstream is not
a remote.

**The headline feature:** live inline (Typora-style) WYSIWYG editing — see
`.claude/skills/livemd/SKILL.md` for the architecture contract. Upstream's
"WYSIWYG" (`ViewMode::Rendered`) is block-level click-to-edit and is a
different thing; both coexist.

## Build

See `.claude/skills/ferrite-build/SKILL.md`. Short version:
`export PATH="$HOME/.cargo/bin:$PATH"` then `cargo check --all-targets`.
Cold builds take many minutes — always background them.

Three tests fail on a clean checkout and are unrelated to any change you make:
`markdown::mermaid::tests::test_subgraph_title_width_expansion`,
`markdown::video_embed::tests::document_parses_bare_youtube_url_paragraph`,
`vcs::git::tests::test_git_service_untracked_file`. A handful of
filesystem-dependent workspace/session tests also flake under parallel runs.
Anything else is yours.

## Ways of working: coordinator + cheap workers

This is a 133k-LOC codebase. Reading it directly is the main token cost, so the
top model coordinates and does not do bulk reading or typing.

**Coordinator (top model) does:** diagnosis, architecture decisions, writing
specs, and final verification of returned work.

**Sub-agents (cheaper models) do:** locating code, implementing decided specs,
running builds and triaging output.

| Need | Agent | Model |
|------|-------|-------|
| "Where is X / what calls Y" | `ferrite-scout` | haiku |
| Implement an already-decided change | `rust-impl` | sonnet |
| Build + clippy + test, triaged | `rust-verify` | sonnet |

Rules that make this actually save tokens rather than just move them around:

- **Never read a large file to find one thing.** Send a scout; it returns
  `file:line` anchors instead of file contents.
- **Decide before delegating.** A spec with open design questions in it comes
  back as a redesign the coordinator has to review line by line — more expensive
  than doing it inline. Bake the decisions in first.
- **Any self-contained ~50+ line change goes to `rust-impl`.** Doing it inline
  because it "feels faster" is the failure mode this structure exists to prevent.
- **Sub-agents report anchors and outcomes, never code back.** The coordinator
  reads the diff if it needs to.
- **Verify returned work; don't trust the report.** Sub-agents overstate success.
  A claimed-passing build gets confirmed before the phase is called done.

## Verify anchors before writing a spec

Twice, a spec built on a plausible-looking anchor sent an agent to the wrong
place:

- `markdown::widgets::EditableHeading` *looks* like the rendered-mode heading
  renderer. Nothing constructs it. The live one is
  `markdown::editor::render_heading`.
- A code comment said `MaxLineWidth` defaulted to `Off`. It defaulted to
  `Col100`, and the real bug was elsewhere.

Grep for construction sites, not just definitions. Treat comments as claims.

## The house rules that have bitten

Each of these was a real, shipped bug:

- **`RichText::strong()` does not bold.** egui documents it as "stronger
  *colour*", and in the light theme it resolves to the same colour as body text
  — a complete no-op. Weights are registered as separate font families; use
  `fonts::chrome_bold_font`.
- **Measure contrast; do not assert brightness.** Doc comments claiming ratios
  were wrong by up to 4 points. `theme::contrast_tests` now holds every pair to
  a floor. Tests should assert the *property* (hue, ordering, ratio), never a
  specific colour — a test asserting "H1 is blue" blocks changing the accent.
- **Typography is per-line, not per-span.** Leading, heading size and block
  spacing are properties of a line. Deciding them per span put inline code on a
  different baseline from the prose around it.
- **Two code paths that compute the same geometry will drift.** Hit-testing
  measured line heights independently of the render loop, so clicks landed
  several lines off. If you must duplicate, add a test pinning the shared
  arithmetic.
- **Changing a `Default` helps nobody who already has a config.** Saved values
  win. Use the versioned migration in `Settings::migrate`, and only rewrite
  values that still equal the old default.

## Code conventions (inherited from upstream — keep them)

- `//!` module-level docs on every module; `///` on public items.
- `// ─────` separator comments between logical sections of large files.
- No `unwrap()` on fallible runtime paths — a panic in the event loop destroys
  the user's unsaved work.
- All user-visible strings go through `t!`; ten locales ship. Verify a key
  actually resolves — a missing one renders as the raw key path.
- Design docs live in `docs/technical/` alongside upstream's.

## Where the design decisions are written down

- `docs/technical/ui-review-2026-08.md` — the UI review and its remediation.
- `docs/technical/settings-polish-2026-08.md` — settings and chrome polish.
- `src/theme/typescale.rs` — the one document type scale. There were three
  independent heading ramps before it, two of them live.
- `tools/iconfont/`, `tools/bodyfont/` — reproducible font pipelines.
