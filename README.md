# Kotori Skrivr

A fast, lightweight Markdown editor with **live inline WYSIWYG editing**. Built with Rust and egui for a native, responsive experience. Also handles JSON, YAML, TOML and CSV.

> **Fork notice.** Kotori Skrivr is a fork of [Ferrite](https://github.com/OlaProeis/Ferrite) by [OlaProeis](https://github.com/OlaProeis), used under the MIT licence. Upstream did the overwhelming majority of the work here; see [LICENSE](LICENSE). Bugs in this fork are ours, not theirs — please report them here rather than upstream.

## What's different from Ferrite

**Live inline WYSIWYG (`LiveMarkdown` view mode).** Upstream's rendered mode is block-level click-to-edit: blocks display styled, and clicking one swaps it back to raw markdown in a text box. Kotori Skrivr adds a genuinely continuous editing surface — text stays styled as you type, and syntax markers are hidden except on the line the cursor is on.

```
cursor NOT on line:   Some bold text here.       <- markers hidden
cursor ON line:       Some **bold** text here.   <- markers revealed
```

Both modes ship; they suit different habits.

**A design pass over the reading and writing experience.** Warm grounds instead of neutral grey, a serif body face, an 80-character centred measure, real leading, and block spacing around headings. Details and reasoning in [`docs/technical/`](docs/technical/).

Also fixed in this fork:

- **Crash recovery could inject content into the wrong file.** An untitled tab's recovery snapshot could be applied to a different, path-backed document that reused its tab id.
- **The test suite did not compile.** 42 errors in test-only modules meant no test in the project could run — which was also hiding 5 genuine failures.
- **Live inline mode was unreachable.** The mode switcher offered three of the four modes, and selecting the fourth drew no selection at all.
- **Selection drifted in live mode.** Hit-testing measured line heights independently of the renderer, so on a document with headings a click landed several lines from the pointer, worsening further down the page.
- **Headings rendered at 2.21:1 contrast**, below the floor for large text — the accent colour was used unchanged as document text. The same value shipped into exported HTML.
- **The editor scanned the whole document twice per frame while typing**, once for a control that no longer existed.
- **Screen readers announced the window controls as `" "`.** Close, minimize, maximize and more had no accessible name; nothing in the interface had a focus ring.

---

## Features

### Core Editing
- **WYSIWYG Markdown Editing** - Edit markdown with live preview, **one-click block switching** between headings, paragraphs, lists, and table cells in rendered mode, click-to-edit formatting, and syntax highlighting
- **Executable Code Blocks** - Run shell or Python fenced blocks from rendered/split preview (`▶ Run`); inline ANSI output, timeout, and Stop (opt-in via Settings; first-run consent)
- **Multi-Format Support** - Native support for Markdown, JSON, CSV, YAML, and TOML files
- **Multi-Encoding Support** - Auto-detect and preserve file encodings (UTF-8, Latin-1, Shift-JIS, Windows-1252, GBK, and more)
- **Tree Viewer** - Hierarchical view for JSON/YAML/TOML with inline editing, expand/collapse, and path copying
- **Find & Replace** - Search with regex support and match highlighting
- **Go to Line (Ctrl+G)** - Quick navigation to specific line number
- **Undo/Redo** - Full undo/redo support per tab

### View Modes
- **Live Inline WYSIWYG** - One continuous editing surface; markers reveal only on the cursor's line
- **Split View** - Side-by-side raw editor and rendered preview with resizable divider; both panes are fully editable; optional **live scroll sync** (minimap **Sync** / **2-way** controls)
- **Zen Mode** - Distraction-free writing with centered text column
- **Position carries across modes** - Switching view keeps your place in the document

### Editor Features
- **Syntax Highlighting** - Full-file syntax highlighting for 100+ languages (Rust, Python, JavaScript, Go, TypeScript, PowerShell, and more)
- **Code Folding** - Fold/unfold regions with gutter indicators (▶/▼) for headings, code blocks, and lists; collapsed content is hidden
- **Semantic Minimap** - Navigation panel with clickable header labels, content type indicators, and text density bars (switchable to VS Code-style pixel view)
- **Multi-Cursor Editing** - Ctrl+Click to add multiple cursors; type, delete, and navigate at all positions simultaneously
- **Bracket Matching** - Highlight matching brackets `()[]{}<>` and emphasis pairs `**` `__`
- **Auto-close Brackets & Quotes** - Type `(`, `[`, `{`, `"`, or `'` to get matching pair; selection wrapping supported
- **Duplicate Line (Ctrl+Shift+D)** - Duplicate current line or selection
- **Move Line Up/Down (Alt+↑/↓)** - Rearrange lines without cut/paste
- **Smart Paste for Links** - Select text then paste URL to create `[text](url)` markdown link
- **Drag & Drop Images** - Drop images into editor to auto-save to `./assets/` and insert markdown link
- **Table of Contents** - Generate/update TOC from headings with `<!-- TOC -->` block (Ctrl+Shift+U)
- **Snippets** - Text expansions like `;date` → current date, `;time` → current time, plus custom snippets
- **Auto-Save** - Configurable auto-save with temp-file safety
- **Line Numbers** - Optional line number gutter
- **Configurable Line Width** - Limit text width for readability (80/100/120 or custom)
- **Typography built for reading** - [Literata](https://github.com/googlefonts/literata) for the document, Inter for the interface, JetBrains Mono for code; an 80-character centred measure and adjustable line height
- **Custom Font Selection** - Choose preferred fonts for editor and UI; important for CJK regional glyph preferences
- **Keyboard Shortcut Customization** - Rebind shortcuts via settings panel

### MermaidJS Diagrams
Native rendering of 11 diagram types directly in the preview:
- Flowchart, Sequence, Pie, State, Mindmap
- Class, ER, Git Graph, Gantt, Timeline, User Journey
- **Insert templates** - Format toolbar → **Insert → Mermaid…** for starter snippets per diagram type
- **Inline validation** - Parse errors in preview with squiggles on broken `mermaid` blocks in the raw editor
- **Flowchart polish** - Improved edge routing, extra shapes, `style` / classDef support; state diagrams support fork/join and history nodes

> **v0.3.0 Mermaid update:** Insert toolbar, syntax help (F1), inline validation, flowchart edge-routing and layout fixes, and state-diagram pseudostates. Complex diagrams may still differ from mermaid.js. See [ROADMAP.md](ROADMAP.md) for planned parity work.

### CSV/TSV Viewer
- **Native Table View** - View CSV and TSV files in a formatted table with fixed-width column alignment
- **Rainbow Column Coloring** - Alternating column colors for improved readability
- **Delimiter Detection** - Auto-detect comma, tab, semicolon, and pipe separators
- **Header Row Detection** - Intelligent detection and highlighting of header rows

### Workspace Features
- **Workspace Mode** - Open folders with file tree, quick switcher (Ctrl+P), and search-in-files (Ctrl+Shift+F)
- **Workspace File Index** - Ctrl+P and Search in Files scan the **entire workspace** in the background (not only expanded folders in the tree)
- **Quick Note Workflow** - Pathless scratch tabs: quit without a save dialog by default; closing a tab still prompts; unsaved notes persist via session recovery (Settings → Files)
- **Git Integration** - Visual status indicators (modified, added, untracked, ignored) with auto-refresh on save, focus, and file changes
- **Session Persistence** - Restore open tabs, cursor positions, and scroll offsets on restart; hardened crash recovery with identity checks and disk-conflict banner

### Terminal Workspace
- **Integrated Terminal** - Multiple instances with shell selection (PowerShell, CMD, WSL, bash)
- **Tiling & Splitting** - Create complex 2D grids with horizontal and vertical splits
- **Smart Maximize** - Temporarily maximize any pane to focus on work (Ctrl+Shift+M)
- **Layout Persistence** - Save and load your favorite terminal arrangements to JSON files
- **Theming & Transparency** - Custom color schemes (Dracula, etc.) and background opacity
- **Drag-and-Drop Tabs** - Reorder terminals with visual feedback
- **AI-Ready** - Visual "breathing" indicator when terminal is waiting for input (perfect for AI agents)

### Additional Features
- **Light & Dark Themes** - Warm paper and warm charcoal grounds, switchable at runtime; **custom accent color** (Settings / Welcome). Every colour pair is held to a WCAG contrast floor by a test, and a badly-chosen accent is corrected rather than rendered unreadable
- **Document Outline & Statistics** - Navigate with outline panel; tabbed statistics showing word count, reading time, heading/link/image counts
- **Export & Print** - Export to **PDF** or **themed HTML** (options dialog, Mermaid as SVG); **print preview** opens a temp PDF in the in-app viewer; copy as HTML
- **Formatting Toolbar** - Quick access to bold, italic, headings, lists, links, Mermaid insert, and more
- **Icon fonts** - Phosphor for interface chrome, plus a purpose-built Skrivr set for the formatting toolbar (built from source artwork by the pipeline in `tools/iconfont/`)
- **Live Pipeline** - Pipe JSON/YAML content through shell commands (for developers)
- **Custom Window** - Borderless window with custom title bar and resize handles
- **Recent Files & Folders** - Click filename in status bar to access recently opened files and workspace folders
- **CJK Paragraph Indentation** - First-line indentation options for Chinese (2 chars) and Japanese (1 char) writing conventions

## Installation

Kotori Skrivr does not publish pre-built binaries yet — build it from source.

```bash
git clone <your-repo-url> kotori-skrivr
cd kotori-skrivr
cargo build --release
./target/release/skrivr
```

Requires Rust 1.92 (pinned in `rust-toolchain.toml`; `rustup` will fetch it automatically).

Upstream Ferrite *does* ship signed Windows installers and packages for Linux and macOS. If you want a ready-made binary and do not need this fork's live WYSIWYG mode, [use Ferrite](https://github.com/OlaProeis/Ferrite/releases) — it is the same editor otherwise.

See [docs/building.md](docs/building.md) for platform-specific build dependencies.

### Installing as a macOS app

`cargo build --release` produces a bare Unix executable. macOS needs an `.app`
bundle before the editor gets its icon, its Dock identity, and the Markdown /
JSON / YAML / TOML file associations declared in
`assets/macos/info_plist_ext.xml`. Building and installing that bundle is one
target:

```bash
make install-macos
```

It quits a running copy, runs `cargo bundle --release`, replaces
`/Applications/Kotori Skrivr.app`, and prints the version it installed. The old
bundle is removed rather than copied over, so files dropped between versions
don't linger as stale resources. Requires [`cargo-bundle`](https://crates.io/crates/cargo-bundle)
(`cargo install cargo-bundle`).

Everything else leaves your installed copy alone. `cargo run --release` and
`./target/release/skrivr` run the bare binary, which has no bundle identifier
and is never registered with macOS — it cannot take over your file
associations or shadow the app in `/Applications`. Only `make install-macos`
touches what's installed, so you choose when a new build becomes your real
Skrivr.

One caveat if you run `cargo bundle` by hand rather than through the target: it
leaves two staging bundles, `target/release/bundle/osx` and
`.../bundle/dmg`, and both carry the same `se.kotori.skrivr` identifier as the
installed copy. macOS registers every `.app` it finds, so you end up with three
bundles claiming one identifier — and LaunchServices then picks between them
unpredictably, which surfaces as Finder refusing to open a `.md` file at all.
`make install-macos` unregisters and deletes them for you. To clean up by hand:

```bash
LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
"$LSREGISTER" -u "$PWD/target/release/bundle/osx/Kotori Skrivr.app"
"$LSREGISTER" -u "$PWD/target/release/bundle/dmg/Kotori Skrivr.app"
rm -rf target/release/bundle
"$LSREGISTER" -f -R -trusted "/Applications/Kotori Skrivr.app"
```

Once installed, Finder's "Open With" and double-clicking an associated file
both open that file in Skrivr — launching the app if it is not running, and
adding a tab to the existing window if it is. Finder delivers those paths as an
Apple Event rather than in `argv`, which the app handles in
`src/platform/macos.rs`.

## Usage

```bash
# Open a file
skrivr path/to/file.md

# Open a folder as workspace
skrivr path/to/folder/
```

<details>
<summary><strong>More CLI options</strong></summary>

```bash
# Run from source
cargo run --release

# Or run the binary directly
./target/release/skrivr

# Open multiple files as tabs
./target/release/skrivr file1.md file2.md

# Show version
./target/release/skrivr --version

# Show help
./target/release/skrivr --help
```

See [docs/cli.md](docs/cli.md) for full CLI documentation.

</details>

### View Modes

Kotori Skrivr supports four view modes for Markdown files:

- **Raw** - Plain text editing with syntax highlighting
- **Rendered** - Block-level WYSIWYG; click a block to edit it as markdown
- **Split** - Side-by-side raw editor and live preview
- **Live** - Continuous inline WYSIWYG: text stays styled as you type, and
  syntax markers hide except on the cursor's line

Cycle with `Cmd/Ctrl+E`, or pick a mode from the labelled switcher in the title
bar. All four share one type scale, so headings do not change size when you
switch, and the reading position carries across.

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New file |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save file |
| `Ctrl+W` | Close tab |
| `Ctrl+P` | Quick file switcher |
| `Ctrl+F` | Find |
| `Ctrl+G` | Go to line |
| `Ctrl+,` | Open settings |

<details>
<summary><strong>All Keyboard Shortcuts</strong></summary>

### File Operations

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New file |
| `Ctrl+O` | Open file |
| `Ctrl+S` | Save file |
| `Ctrl+Shift+S` | Save as |
| `Ctrl+W` | Close tab |

### Navigation

| Shortcut | Action |
|----------|--------|
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+P` | Quick file switcher (workspace) |
| `Ctrl+Shift+F` | Search in files (workspace) |

### Editing

| Shortcut | Action |
|----------|--------|
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Ctrl+F` | Find |
| `Ctrl+H` | Find and replace |
| `Ctrl+G` | Go to line |
| `Ctrl+Shift+D` | Duplicate line |
| `Alt+↑` | Move line up |
| `Alt+↓` | Move line down |
| `Ctrl+B` | Bold |
| `Ctrl+I` | Italic |
| `Ctrl+K` | Insert link |

### View

| Shortcut | Action |
|----------|--------|
| `F11` | Toggle fullscreen |
| `Ctrl+E` | Cycle view mode (Raw / Rendered / Split / Live) |
| `Ctrl+,` | Open settings |
| `Ctrl+Shift+[` | Fold all |
| `Ctrl+Shift+]` | Unfold all |

### Terminal Workspace

Terminal shortcuts are **context-aware**; they work when the terminal panel is focused.

| Shortcut | Action |
|----------|--------|
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Cycle through terminal tabs |
| `Ctrl+1-9` | Switch to specific terminal tab |
| `Ctrl+Arrow Keys` | Move focus between split panes |
| `Ctrl+Shift+M` | Toggle **Maximize Pane** (Zoom) |
| `Ctrl+L` | Clear terminal screen |
| `Ctrl+Shift+C` | Copy selection / screen |
| `Ctrl+Shift+V` | Paste to terminal |
| `Ctrl+W` / `Ctrl+F4` | Close focused pane (auto-collapses splits) |
| `Double-click Tab` | Rename terminal |

</details>

## Configuration

Access settings via `Ctrl+,` or the gear icon. Configure appearance, editor behavior, and file handling.

<details>
<summary><strong>Configuration details</strong></summary>

Settings are stored in platform-specific locations:

- **Windows:** `%APPDATA%\skrivr\`
- **Windows Portable:** `portable\` folder next to `skrivr.exe`
- **Linux:** `~/.config/skrivr/`
- **macOS:** `~/Library/Application Support/skrivr/`

**Portable Mode (Windows):** If a `portable` folder exists next to the executable, Kotori Skrivr automatically uses it for all configuration instead of `%APPDATA%`. This makes Kotori Skrivr fully self-contained - perfect for USB drives.

Workspace settings are stored in `.skrivr/` within the workspace folder.

### Settings Panel

- **Appearance:** Theme, font family, font size, default view mode
- **Editor:** Word wrap, line numbers, minimap, bracket matching, code folding, syntax highlighting, auto-close brackets, line width
- **Files:** Auto-save, recent files history

</details>

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned features and known issues.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Help Translate

Kotori Skrivr is being translated into multiple languages with help from the community.

[![Translation Status](https://hosted.weblate.org/widget/skrivr/skrivr-ui/multi-auto.svg)](https://hosted.weblate.org/engage/skrivr/)

**[Help translate Kotori Skrivr on Weblate](https://hosted.weblate.org/engage/skrivr/)** - no coding required!

<details>
<summary><strong>Quick Start for Contributors</strong></summary>

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/Kotori Skrivr.git
cd Kotori Skrivr

# Create a feature branch
git checkout -b feature/your-feature

# Make changes, then verify
cargo fmt
cargo clippy
cargo test
cargo build

# Optional (Nix users): validate flake outputs
nix flake check

# Commit and push
git commit -m "feat: your feature description"
git push origin feature/your-feature
```

</details>

## Tech Stack

Built with Rust 1.70+, egui/eframe for GUI, comrak for Markdown parsing, ropey for rope-based text editing, and syntect for syntax highlighting.

<details>
<summary><strong>Full tech stack</strong></summary>

| Component | Technology |
|-----------|------------|
| Language | Rust 1.70+ |
| GUI Framework | egui 0.28 + eframe 0.28 |
| Text Buffer | ropey 1.6 (rope data structure) |
| Markdown Parser | comrak 0.22 |
| Syntax Highlighting | syntect 5.1 + two-face 0.5 |
| Git Integration | git2 0.19 |
| Terminal PTY | portable-pty 0.8 |
| Terminal ANSI Parser | vte 0.13 |
| Encoding Detection | encoding_rs 0.8 + chardetng 0.1 |
| Internationalization | rust-i18n 3 + sys-locale 0.3 |
| CLI Parsing | clap 4 |
| File Dialogs | rfd 0.14 |
| Clipboard | arboard 3 |
| File Watching | notify 6 |
| Fuzzy Matching | fuzzy-matcher 0.3 |
| HTTP Client | ureq 2 (update checker) |
| Hashing | blake3 1.5 (Mermaid caching) |
| Date/Time | chrono 0.4 |
| CSV Parsing | csv 1.3 |
| Color Palette | palette 0.7 |
| Font Enumeration | font-kit 0.14 |
| Allocator (Windows) | mimalloc 0.1 |
| Allocator (Unix) | tikv-jemallocator 0.6 |

</details>

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

### Libraries
- [egui](https://github.com/emilk/egui) and eframe - Immediate mode GUI and native window integration
- [ropey](https://github.com/cessen/ropey) - Rope text buffer for large-file editing
- [comrak](https://github.com/kivikakk/comrak) - CommonMark + GFM compatible Markdown parser
- [syntect](https://github.com/trishume/syntect) and [two-face](https://github.com/CosmicHorrorDev/two-face) - Syntax highlighting and extra language definitions
- [harfrust](https://github.com/harfbuzz/harfrust) - OpenType text shaping for complex scripts
- [git2](https://github.com/rust-lang/git2-rs) - libgit2 bindings for Rust
- [portable-pty](https://github.com/wez/wezterm) and [vte](https://github.com/alacritty/vte) - Integrated terminal (PTY and ANSI parsing)
- [image](https://github.com/image-rs/image) - Raster image decoding (markdown preview and image viewer)
- [hayro](https://github.com/LaurenzV/hayro) - Pure Rust PDF rasterization (PDF viewer tabs)
- [rust-i18n](https://github.com/longbridge/rust-i18n) - Internationalization
- [Inter](https://rsms.me/inter/) and [JetBrains Mono](https://www.jetbrains.com/lp/mono/) fonts

### Development Tools
- [Claude](https://anthropic.com) (Anthropic) - AI assistant that wrote the code
- [Cursor](https://cursor.com) - AI-powered code editor
- [Task Master](https://github.com/eyaltoledano/claude-task-master) - AI task management for development workflows

### Contributors
- [@Star-sumi](https://github.com/Star-sumi) — Windows single-instance foreground activation when opening files from Explorer ([PR #148](https://github.com/OlaProeis/Ferrite/pull/148), fixes [#147](https://github.com/OlaProeis/Ferrite/issues/147))
- [@moabtools](https://github.com/moabtools) — Ctrl+Home / Ctrl+End document navigation in Rendered view ([PR #137](https://github.com/OlaProeis/Ferrite/pull/137))
- [@liuxiaopai-ai](https://github.com/liuxiaopai-ai) — Nix/NixOS flake support for reproducible builds and dev shells ([PR #92](https://github.com/OlaProeis/Ferrite/pull/92))
- [@blizzard007dev](https://github.com/blizzard007dev) — Welcome page for first-launch configuration ([PR #80](https://github.com/OlaProeis/Ferrite/pull/80))
- [@wolverin0](https://github.com/wolverin0) — Integrated Terminal Workspace & Productivity Hub ([PR #74](https://github.com/OlaProeis/Ferrite/pull/74))
- [@abcd-ca](https://github.com/abcd-ca) — Delete Line, Move Line, macOS file associations ([PR #29](https://github.com/OlaProeis/Ferrite/pull/29), [#30](https://github.com/OlaProeis/Ferrite/pull/30))
- [@SteelCrab](https://github.com/SteelCrab) — CJK character rendering ([PR #8](https://github.com/OlaProeis/Ferrite/pull/8))

## Sponsors

<table>
  <tr>
    <td>
      <a href="https://signpath.io/?utm_source=foundation&utm_medium=github&utm_campaign=skrivr" target="_blank"><img src="https://signpath.org/assets/favicon-50x50.png" alt="SignPath" width="50" height="50" /></a>
    </td>
    <td>
      Free code signing on Windows provided by <a href="https://signpath.io/?utm_source=foundation&utm_medium=github&utm_campaign=skrivr">SignPath.io</a>, certificate by <a href="https://signpath.org/?utm_source=foundation&utm_medium=github&utm_campaign=skrivr">SignPath Foundation</a>
    </td>
  </tr>
  <tr>
    <td>
      <a href="https://weblate.org/" target="_blank"><img src="https://weblate.org/static/img/logo.svg" alt="Weblate" width="50" height="50" /></a>
    </td>
    <td>
      Hosted translations provided by <a href="https://weblate.org/">Weblate</a> on <a href="https://hosted.weblate.org/">Hosted Weblate</a> (gratis libre project plan)
    </td>
  </tr>
</table>

---

<sub>If you find Kotori Skrivr useful, consider [supporting its development](https://github.com/sponsors/OlaProeis).</sub>
