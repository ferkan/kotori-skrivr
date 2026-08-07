---
name: ferrite-build
description: Build, check, lint, run and test this Rust/egui markdown editor. Use before running any cargo command in this repo, and whenever a build fails in a way that looks environmental rather than code-related.
---

# Building this project

Rust/egui desktop app, ~133k LOC. Pinned to Rust **1.92.0** via `rust-toolchain.toml`.

## PATH — required

`rustup` was installed to the user's home, not system-wide. Every shell you spawn
must export the path first or `cargo` will not be found:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Commands

| Task | Command |
|------|---------|
| Fast compile check (**use this in inner loops**) | `cargo check --all-targets` |
| Debug build | `cargo build` |
| Run the app | `cargo run` |
| Run on a file | `cargo run -- test_md/test.md` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format | `cargo fmt` |
| Tests | `cargo test` |

`make check`, `make lint`, `make precommit` wrap the same things.

## Timing — plan around this

A cold build compiles the full egui/eframe/wgpu-adjacent dependency tree and takes
**many minutes**. Warm incremental `cargo check` is seconds.

Consequence: **never run a cold build in the foreground.** Use
`run_in_background: true` and poll the output file. Prefer `cargo check` over
`cargo build` unless you specifically need a runnable binary — it skips codegen
and is dramatically faster.

## Platform note

Upstream Ferrite is developed on Windows/Linux; **macOS support is explicitly
experimental**. On macOS, treat windowing, IME and Gatekeeper oddities as
pre-existing upstream conditions, not as regressions from our changes — verify
against a clean checkout before claiming we broke something.

## Feature flags

Default features: `bundle-icon`, `high-perf-alloc`. Optional: `async-workers`,
`lsp` (incomplete upstream, deferred — do not enable it expecting it to work).

Building without the icon asset or allocators:
`cargo build --no-default-features`
