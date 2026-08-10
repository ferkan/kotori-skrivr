# Security Audit and Remediation Action Plan — August 2026

**Status:** Open
**Scope:** Kotori Skrivr application, local integrations, persistence, and CI/release pipeline
**Audit type:** Read-only static review with call-site validation
**Last reviewed:** 2026-08-09

## Purpose

This document turns the August 2026 security review into a staged remediation
programme. It is the master checklist for prioritisation and ownership. Each
stage is independently releasable, but Stage 0 is a release gate and should be
completed before publishing binaries from the current workflow.

The review found no unauthenticated remote-code-execution path in the normal
native editor flow. The most serious risks are concentrated in:

- release workflow trust and artifact integrity;
- HTML export of untrusted Markdown;
- local IPC, subprocess, temporary-file, and resource-limit boundaries;
- workspace-controlled files and plaintext recovery data.

## How to use this plan

For every task:

- assign an owner and target milestone before starting;
- create a focused implementation plan with exact code and test changes;
- implement using tests first where the behavior is testable;
- record the pull request or commit in the completion matrix;
- satisfy every acceptance criterion before checking the task complete;
- run the stage exit gate before moving to the next stage.

Do not combine unrelated stages into one large security change. Release
workflow changes, HTML sanitisation, IPC, process containment, and persistence
permissions have different failure modes and should remain separately
reviewable.

## Severity and priority

| Priority | Meaning | Target |
|---|---|---|
| P0 | Release integrity or high-impact data exposure | Before the next public release |
| P1 | Meaningful local security or availability boundary | Next security milestone |
| P2 | Privacy hardening or defense in depth | Planned follow-up |
| P3 | Low-risk hygiene or currently unreachable advisory | Maintenance backlog |

Severity describes impact. Priority also accounts for exploit preconditions,
reachability, and whether the feature is disabled or user-triggered.

## Programme overview

| Stage | Focus | Tasks | Exit condition |
|---|---|---|---|
| 0 | Release integrity gate | SEC-001–SEC-003 | Release inputs are validated, dependencies are pinned, and exact artifacts are verified before signing |
| 1 | Untrusted document export | SEC-004–SEC-005 | Exported HTML cannot silently execute raw document HTML or embed files outside the approved root |
| 2 | Runtime containment | SEC-006–SEC-010 | IPC, images, code execution, temporary files, and workspace metadata have explicit trust and resource boundaries |
| 3 | Privacy and dependency hygiene | SEC-011–SEC-013 | Sensitive persistence is permission-restricted, logs are redacted, and advisory scanning is enforced |
| 4 | Defense in depth | SEC-014–SEC-015 | Navigation and parser attack surfaces have documented policies and regression coverage |

---

## Stage 0 — Release integrity gate

This stage blocks public releases. The existing release workflow is both
injectable through tag names and internally inconsistent about artifact names.

### SEC-001 — Prevent release-tag command injection

**Priority:** P0
**Severity:** High
**Precondition:** Attacker can push a matching `v*` tag
**Impact:** Arbitrary commands on release runners; unsigned artifacts can be
modified before signing and publication.

**Evidence:**

- `.github/workflows/release.yml:13-16` triggers on every `v*` tag.
- `.github/workflows/release.yml:89` inserts `github.ref_name` directly into
  PowerShell source.
- `.github/workflows/release.yml:398` inserts it directly into Bash source.

**Actions:**

- [ ] Pass the tag name through a step-level environment variable instead of
      template-inserting it into `run:` source.
- [ ] Reject tags that do not match the project’s chosen strict SemVer grammar.
- [ ] Ensure the validated version—not the raw tag—is used in file names,
      installer metadata, release titles, and prerelease decisions.
- [ ] Add repository rules that restrict creation and modification of `v*`
      tags to release maintainers.
- [ ] Add a workflow regression check covering `$()`, backticks, quotes,
      newlines, path separators, and option-like version strings.

**Acceptance criteria:**

- A malicious-but-Git-valid tag is rejected before build or signing begins.
- No `${{ github.ref_name }}` expression occurs inside a `run:` block.
- A valid stable tag and a valid prerelease tag produce the expected version.

### SEC-002 — Pin and isolate release supply-chain inputs

**Priority:** P0
**Severity:** High
**Impact:** A compromised mutable Action or live package can alter artifacts,
steal workflow credentials, or misuse the signing identity.

**Evidence:**

- Actions use mutable refs such as `actions/checkout@v4`,
  `signpath/github-action-submit-signing-request@v2`, and
  `softprops/action-gh-release@v1`.
- `.github/workflows/release.yml:76-120`, `:193-196`, `:247-248`, and
  `:300-301` install release tools without exact versions or lock enforcement.
- Build jobs do not declare explicit least-privilege permissions, and checkout
  credentials are persisted by default.

**Actions:**

- [ ] Pin every GitHub Action in CI, Nix, and release workflows to a reviewed
      full commit SHA; retain the human-readable release tag in a comment.
- [ ] Pin exact versions of `cargo-wix`, `cargo-deb`,
      `cargo-generate-rpm`, and `cargo-bundle`; install with `--locked`.
- [ ] Pin Pillow through a hash-locked requirements file or reviewed wheel.
- [ ] Pin the NSIS distribution/version instead of installing the current
      Chocolatey package at release time.
- [ ] Set explicit `permissions: { contents: read }` on all build jobs.
- [ ] Set `persist-credentials: false` on every checkout used for builds.
- [ ] Keep signing credentials out of build jobs and expose them only to the
      dedicated signing step.
- [ ] Add reviewed dependency-update automation for pinned Actions and tools.

**Acceptance criteria:**

- The release workflow has no mutable Action refs or unversioned tool installs.
- Build steps cannot push to the repository or publish releases.
- Only the signing job can request a signing operation.
- A clean runner can reproduce the same tool versions from repository data.

### SEC-003 — Establish one canonical artifact and signing map

**Priority:** P0
**Severity:** High availability / release-integrity blocker
**Impact:** Releases fail, package stale files, or submit the wrong executable
for signing.

**Evidence:**

- `Cargo.toml:12-14` produces the `skrivr` binary.
- `.github/workflows/release.yml:42`, `:114`, and `:204` expect `ferrite`.
- `portable/installer.nsi:67-68` expects `Kotori Skrivr/skrivr.exe`.
- `portable/.../FerriteMDPortable.ini:2` expects `Ferrite/ferrite.exe`.
- `.signpath/artifact-configuration.xml:16-25` describes only the portable ZIP
  and MSI, while the workflow also labels the PortableApps installer signed.

**Actions:**

- [ ] Choose `skrivr` as the canonical executable basename on every platform.
- [ ] Update archive, MSI, PortableApps, macOS bundle, and Linux package paths
      to the canonical map.
- [ ] Extend signing configuration to every Windows artifact represented as
      signed, or label unsupported artifacts clearly as unsigned.
- [ ] Verify Authenticode signatures after SignPath returns artifacts.
- [ ] Generate SHA-256 checksums and provenance/attestations for every public
      artifact.
- [ ] Fail the workflow before upload if an expected file is absent, has the
      wrong basename, or does not match the recorded build hash.

**Acceptance criteria:**

- All packaging steps consume the binary produced by Cargo without manual
  renaming outside the documented map.
- The signing configuration and release asset list contain the same artifacts.
- Each published asset has a verified signature where supported and a published
  checksum.

### Stage 0 exit gate

- [ ] Validate workflow syntax with the repository’s selected workflow linter.
- [ ] Exercise stable, prerelease, and rejected malicious tag cases in a safe
      test repository or non-publishing workflow.
- [ ] Produce all platform artifact names from one release candidate.
- [ ] Confirm the Windows signatures and all published checksums independently.
- [ ] Require security review of the workflow diff before the next release tag.

---

## Stage 1 — Untrusted document export

Stage 1 treats Markdown as untrusted input. Native preview currently displays
raw HTML as a marker; the dangerous boundary is HTML generation and subsequent
opening, pasting, or sharing.

### SEC-004 — Make HTML export safe by default

**Priority:** P0
**Severity:** High
**Precondition:** User exports or copies attacker-controlled Markdown as HTML
**Impact:** Stored script execution or active-content injection in the browser
or receiving HTML-capable application.

**Evidence:**

- `src/export/html.rs:769-789` enables Comrak unsafe rendering.
- `src/export/html.rs:921-930` enables the same behavior for clipboard HTML.
- `src/app/export.rs:124-132` can open the generated document.

**Actions:**

- [ ] Define one shared HTML safety policy for file and clipboard exports.
- [ ] Disable raw/unsafe HTML in the default policy.
- [ ] If raw HTML export remains available, make it an explicit advanced option
      with a warning and sanitize it through a maintained allowlist sanitizer.
- [ ] Allow only approved URL schemes and strip inline event handlers,
      scripts, active embeds, dangerous SVG, and unsafe `data:` uses.
- [ ] Add a restrictive Content Security Policy to standalone HTML exports.
- [ ] Ensure Mermaid output and syntax highlighting do not require broad script
      or style exceptions.

**Regression cases:**

- [ ] `<script>` is removed or escaped.
- [ ] `onerror`, `onclick`, and equivalent event attributes are removed.
- [ ] `javascript:`, unsafe `data:`, `iframe`, `object`, and active SVG payloads
      do not survive the safe policy.
- [ ] Safe headings, tables, code blocks, links, and generated diagrams retain
      their expected output.

**Acceptance criteria:**

- Default HTML export and copy-as-HTML contain no executable document-supplied
  content.
- Any unsafe/raw mode is explicit, persistent only by deliberate choice, and
  clearly marked as active content.

### SEC-005 — Confine self-contained export file embedding

**Priority:** P0
**Severity:** High when combined with SEC-004
**Impact:** Arbitrary local files can be copied into an exported document and
then disclosed through opening or sharing it.

**Evidence:**

- `src/export/html_options.rs:62-75` enables self-contained export by default.
- `src/export/html.rs:364-397` reads every non-network image `src` without
  canonical containment or validating that the bytes are an image.

**Actions:**

- [ ] Canonicalize the document directory, configured base directory, and every
      candidate asset before reading it.
- [ ] Reject assets outside the approved base by default, including `..`,
      absolute paths, `file:` paths, and symlink escapes.
- [ ] Validate decoded image format instead of trusting the extension.
- [ ] Enforce per-asset and total-export byte limits.
- [ ] Present an explicit file list and confirmation if outside-root embedding
      is intentionally supported.
- [ ] Escape or structurally rewrite generated `src` attributes rather than
      relying on a regex over arbitrary HTML.

**Regression cases:**

- [ ] Relative in-root PNG/JPEG assets embed successfully.
- [ ] Absolute paths, `../` escapes, symlink escapes, and non-image files fail
      closed.
- [ ] Raw `<img>` HTML cannot bypass the same policy.
- [ ] Oversized individual and aggregate assets produce a clear export error.

### Stage 1 exit gate

- [ ] Run focused HTML export and clipboard tests.
- [ ] Manually inspect a generated safe document in a browser with developer
      tools open and confirm CSP enforcement.
- [ ] Test the combined local-file-plus-script proof of concept and confirm both
      parts are blocked.
- [ ] Confirm ordinary Markdown export remains visually usable.

---

## Stage 2 — Runtime containment

### SEC-006 — Authenticate and bound single-instance IPC

**Priority:** P1
**Severity:** Medium
**Precondition:** Attacker has local access and can discover or scan the port
**Impact:** Forced file/workspace opens, focus disruption, and memory/CPU denial
of service.

**Evidence:** `src/single_instance.rs:60-159` accepts unauthenticated localhost
TCP lines; `src/app/file_ops.rs:1475-1550` trusts resulting paths.

**Actions:**

- [ ] Prefer a permission-protected Unix-domain socket and Windows named pipe;
      otherwise authenticate TCP requests with a random per-instance token.
- [ ] Store endpoint metadata and tokens with owner-only permissions.
- [ ] Define maximum connection bytes, line bytes, path count, and queue depth.
- [ ] Canonicalize and deduplicate paths before queuing UI work.
- [ ] Separate focus-only requests from file/workspace requests.
- [ ] Decide whether workspace-open requests require user confirmation.
- [ ] Add malformed, oversized, unauthenticated, and flood tests.

**Acceptance criteria:** Unauthenticated or oversized requests are rejected
without allocating proportionally to attacker-controlled input or changing UI
state.

### SEC-007 — Add image decoding budgets and path policy

**Priority:** P1
**Severity:** Medium
**Impact:** A referenced decompression-bomb or huge image can freeze or terminate
the process; absolute and escaped paths cross the workspace boundary.

**Evidence:** `src/markdown/editor.rs:5902-5946` accepts broad local paths;
`:5949-5976` reads, decodes, expands to RGBA, duplicates pixels, and uploads a
texture without limits.

**Actions:**

- [ ] Define maximum encoded bytes, width, height, decoded pixels, animation
      frames, and aggregate texture memory per document.
- [ ] Inspect dimensions and enforce decoder limits before allocating RGBA.
- [ ] Canonicalize local paths and default to document/workspace containment.
- [ ] Require confirmation or a setting for intentionally external images.
- [ ] Cache failures without retaining attacker-sized input.
- [ ] Add high-expansion, huge-dimension, malformed, symlink, and path-escape
      regression fixtures of minimal repository size.

**Acceptance criteria:** Every rejected image fails with a bounded allocation
and a non-fatal placeholder; approved normal images still render.

### SEC-008 — Contain code-runner processes and output

**Priority:** P1
**Severity:** Medium
**Precondition:** Code execution is enabled and the user runs a block
**Impact:** Descendant processes survive timeout; inherited pipes can hang
reader joins; output can exhaust memory.

**Evidence:** `src/markdown/code_execution.rs:523-583` kills only the immediate
child and joins readers; `:586-605` grows stdout/stderr without a limit.

**Actions:**

- [ ] Launch each run in a Unix process group or Windows Job Object.
- [ ] On cancel, timeout, or editor shutdown, terminate and reap the complete
      process tree.
- [ ] Enforce separate and combined stdout/stderr byte limits with truncation
      status surfaced to the UI.
- [ ] Bound concurrent code runs globally and per document.
- [ ] Ensure reader shutdown cannot block indefinitely when descendants retain
      handles.
- [ ] Add tests for background descendants, output floods, timeout, cancel,
      normal exit, and application shutdown.

**Acceptance criteria:** No descendant or reader thread remains after the
configured deadline, and memory use stays within the documented output budget.

### SEC-009 — Replace predictable temporary files

**Priority:** P1
**Severity:** Low/Medium
**Impact:** Symlink overwrite, race, script disclosure, or script substitution
on shared temporary directories.

**Evidence:**

- `src/diag.rs:34-75` uses fixed temporary names and follows symlinks.
- `src/markdown/code_execution.rs:375-386` uses a timestamp-derived script
  path without exclusive creation.
- `src/app/export.rs:324-330` does the same for print-preview PDFs.

**Actions:**

- [ ] Use `tempfile::TempDir` or `NamedTempFile` for diagnostics, scripts, and
      print preview.
- [ ] Create files exclusively with owner-only permissions where supported.
- [ ] Keep the file handle alive through the consumer’s open operation when the
      platform permits it.
- [ ] Remove temporary files on normal close, error, and cancellation.
- [ ] Avoid a global predictable diagnostics filename; expose the generated
      path through logs or UI instead.
- [ ] Add symlink/pre-existence and cleanup regression tests.

**Acceptance criteria:** An attacker-created file or symlink cannot redirect,
replace, or observe newly created temporary content.

### SEC-010 — Treat workspace terminal metadata as untrusted

**Priority:** P1
**Severity:** Medium integrity / Low availability
**Impact:** A workspace symlink can redirect layout writes; crafted layouts can
spawn excessive shells or use arbitrary working directories.

**Evidence:**

- `src/config/settings.rs:2726-2727` enables automatic layout load/save.
- `src/ui/terminal_panel.rs:335-405` loads `.ferrite/terminal-layout.json`.
- `src/ui/terminal_panel.rs:410-470` follows the destination with `fs::write`.
- `src/app/navigation.rs:431-434` saves automatically when hiding the panel.

**Actions:**

- [ ] Canonicalize `.ferrite`, reject symlinked parents/destinations, and enforce
      containment beneath the workspace root before reading or writing.
- [ ] Use atomic no-follow writes for layout persistence.
- [ ] Cap saved tabs, terminals, floating windows, title length, and JSON bytes.
- [ ] Validate every restored cwd against the workspace policy.
- [ ] Do not auto-start workspace-defined terminals before the workspace is
      trusted; alternatively default auto-load off and prompt once per root.
- [ ] Consolidate the legacy root `terminal_layout.json` and current
      `.ferrite/terminal-layout.json` behaviors into one documented path.
- [ ] Add symlink-overwrite, oversized-layout, excessive-terminal, invalid-cwd,
      and trusted-workspace tests.

**Acceptance criteria:** Opening or hiding a terminal in an untrusted workspace
cannot write outside that workspace or create an unbounded number of processes.

### Stage 2 exit gate

- [ ] Run focused IPC, image, code-execution, temporary-file, and terminal-layout
      tests serially where global process/filesystem state is involved.
- [ ] Confirm cancel and shutdown leave no child processes or temporary files.
- [ ] Measure peak memory for rejected image and output-flood fixtures.
- [ ] Run `cargo check --all-targets` using the repository build instructions.

---

## Stage 3 — Privacy and dependency hygiene

### SEC-011 — Restrict persisted sensitive data

**Priority:** P2
**Severity:** Medium privacy, environment-dependent
**Impact:** Unsaved documents, commands, macros, and recent pipeline history may
be readable in permissive or shared configurations.

**Evidence:**

- `src/app/mod.rs:802-824` periodically persists unsaved state.
- `src/config/session.rs:785-837` stores full recovery buffers as JSON.
- `src/config/session.rs:1299-1359` stores full autosave content.
- `src/config/persistence.rs:268-291` does not explicitly set owner-only modes.

**Actions:**

- [ ] Create config, session, recovery, autosave, snippet, IPC metadata, and
      terminal-layout files with the narrowest platform-appropriate permissions.
- [ ] Repair overly broad permissions on existing files during migration without
      following symlinks.
- [ ] Document that recovery and command histories contain plaintext.
- [ ] Provide a per-document “do not persist recovery” control for sensitive
      documents, or a clearly scoped privacy mode.
- [ ] Define retention and cleanup behavior for recovery/autosave data.
- [ ] Add Unix mode tests and platform-specific ACL tests where CI supports them.

**Acceptance criteria:** Newly written sensitive application-state files are
owner-only by default, and privacy mode leaves no recoverable document content.

### SEC-012 — Redact sensitive values from logs

**Priority:** P2
**Severity:** Low
**Impact:** Debug logs can expose URL credentials/query tokens and full paths.

**Evidence:** `src/markdown/widgets.rs:4680-4849` logs complete URLs;
`src/main.rs:281-283` logs CLI paths; image errors log the supplied URL/path.

**Actions:**

- [ ] Centralize URL redaction: remove userinfo, query, and fragment before
      logging.
- [ ] Log path basenames or stable hashes unless a diagnostic mode explicitly
      requires full paths.
- [ ] Ensure errors shown to users remain actionable without copying secrets to
      persistent diagnostics.
- [ ] Add redaction tests for credentials, bearer-like query parameters,
      fragments, Unicode URLs, and local paths.

**Acceptance criteria:** Default and debug logs contain no URL secrets or full
document contents; diagnostic exceptions are explicit and documented.

### SEC-013 — Enforce dependency advisory scanning

**Priority:** P2
**Severity:** Maintenance risk
**Evidence:** `Cargo.lock` pins `git2` 0.19.0. RustSec
RUSTSEC-2026-0008 reports a `git2::Buf` soundness issue fixed in 0.20.4. The
current application does not directly use the affected `Buf` API, so
reachability has not been established.

**Actions:**

- [ ] Add `cargo audit` or an equivalent RustSec-compatible scanner to CI.
- [ ] Fail CI for reachable vulnerabilities; require a reviewed, expiring
      exception for unreachable or informational advisories.
- [ ] Upgrade `git2` to a supported release containing the fix and rerun local
      repository discovery/status tests.
- [ ] Track deprecated/unmaintained dependencies such as `serde_yaml` separately
      from exploitable advisories.
- [ ] Generate a dependency inventory or SBOM for release artifacts.
- [ ] Schedule recurring dependency review and lockfile updates.

**Acceptance criteria:** Every build is checked against a current advisory
database, exceptions record reachability and expiry, and releases include a
dependency inventory.

### Stage 3 exit gate

- [ ] Verify permissions on a clean standard install and portable install.
- [ ] Verify privacy-mode cleanup after clean exit and simulated crash.
- [ ] Run log-redaction tests at every supported log level.
- [ ] Run the dependency scanner against the complete committed lockfile.
- [ ] Review every advisory result for actual call-site reachability.

---

## Stage 4 — Defense in depth

### SEC-014 — Define navigation containment and confirmation policy

**Priority:** P3
**Severity:** Low
**Impact:** Clicked wikilinks can resolve absolute or parent-relative paths and
open arbitrary readable files outside the workspace.

**Evidence:** `src/app/file_ops.rs:2058-2097` joins the target without canonical
containment; navigation is user-click-triggered.

**Actions:**

- [ ] Canonicalize resolved wikilink candidates.
- [ ] Mark links that leave the workspace and require confirmation before open.
- [ ] Prevent invisible label/target mismatches from hiding an external path.
- [ ] Add absolute, parent-relative, symlink, same-workspace, and missing-link
      tests.

**Acceptance criteria:** External navigation is visible and deliberate while
normal in-workspace wikilinks remain one-click actions.

### SEC-015 — Maintain parser and native-library assurance

**Priority:** P3
**Severity:** Defense in depth
**Scope:** Markdown, Mermaid, JSON/YAML/TOML, CSV, image, PDF, font, terminal
escape, and Git-status parsers.

**Actions:**

- [ ] Document input-size and recursion limits for every parser boundary.
- [ ] Add small fuzz targets for custom Mermaid, frontmatter, wikilink, and
      terminal escape parsers.
- [ ] Maintain a minimized malformed-input corpus for image and PDF viewers.
- [ ] Confirm that remote images remain disabled unless a future design adds an
      explicit network trust policy.
- [ ] Keep updater activation behind HTTPS origin validation, signed artifacts,
      and explicit user action.
- [ ] Review unsafe/native transitive dependencies during every major upgrade.

**Acceptance criteria:** Parser budgets are documented and enforced, fuzzing is
repeatable, and any future network content path requires a separate security
design review.

---

## Completion matrix

Update this table when work begins and when acceptance criteria pass.

| ID | Owner | Milestone | Status | PR/commit | Verification evidence |
|---|---|---|---|---|---|
| SEC-001 | Unassigned | Stage 0 | Open | — | — |
| SEC-002 | Unassigned | Stage 0 | Open | — | — |
| SEC-003 | Unassigned | Stage 0 | Open | — | — |
| SEC-004 | Unassigned | Stage 1 | Open | — | — |
| SEC-005 | Unassigned | Stage 1 | Open | — | — |
| SEC-006 | Unassigned | Stage 2 | Open | — | — |
| SEC-007 | Unassigned | Stage 2 | Open | — | — |
| SEC-008 | Unassigned | Stage 2 | Open | — | — |
| SEC-009 | Unassigned | Stage 2 | Open | — | — |
| SEC-010 | Unassigned | Stage 2 | Open | — | — |
| SEC-011 | Unassigned | Stage 3 | Open | — | — |
| SEC-012 | Unassigned | Stage 3 | Open | — | — |
| SEC-013 | Unassigned | Stage 3 | Open | — | — |
| SEC-014 | Unassigned | Stage 4 | Open | — | — |
| SEC-015 | Unassigned | Stage 4 | Open | — | — |

## Verified non-findings and existing safeguards

These checks reduce noise in future reviews:

- Native rendered Markdown does not execute raw HTML; it displays an HTML
  marker.
- Native remote images are not fetched; HTTP, HTTPS, and data images render as
  placeholders.
- Native link opening is restricted to HTTP and HTTPS before calling the OS.
- Code execution is disabled by default and requires explicit consent and a Run
  action.
- LSP is disabled by default; executable overrides are global user settings,
  not workspace Markdown.
- Git integration performs local discovery and status reads; no fetch, push,
  checkout, or credential flow was found.
- The updater is disabled and currently downloads no executable.
- No archive extraction path was found in application code.

These safeguards must be preserved by regression tests when adjacent code is
changed.

## Audit limitations

- The review was static and read-only; no exploit payload was executed.
- `cargo-audit` and `cargo-deny` were not installed, so a complete transitive
  advisory scan was not performed.
- Platform-specific permission, named-pipe, process-group, Job Object, signing,
  and installer behavior requires validation on the relevant operating system.
- Severity assumes Markdown files and workspaces may originate from untrusted
  people or repositories. Features requiring explicit user action are called
  out in each task.

## Recommended implementation sequence

1. SEC-001 and SEC-003 together, because the release workflow must first build
   and name the correct artifacts safely.
2. SEC-002 before those artifacts are trusted or signed.
3. SEC-004 and SEC-005 together, because either partial fix leaves the combined
   export attack chain incompletely addressed.
4. SEC-006, SEC-007, SEC-008, SEC-009, and SEC-010 as separate focused changes.
5. SEC-011 through SEC-013 as the privacy and maintenance milestone.
6. SEC-014 and SEC-015 after the externally meaningful boundaries are closed.
