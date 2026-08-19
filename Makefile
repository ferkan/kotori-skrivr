# Kotori Skrivr Build Makefile
# Simple build automation for local development

.PHONY: build release run clean check fmt clippy test all install-macos

# Default target
all: check build

# Debug build (fast compilation)
build:
	cargo build

# Release build (optimized)
release:
	cargo build --release

# Run debug build
run:
	cargo run

# Run release build
run-release:
	cargo run --release

# Check for errors without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Run clippy lints
clippy:
	cargo clippy --all-targets -- -D warnings

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Full lint check (format + clippy)
lint: fmt clippy

# Pre-commit check (format, clippy, test, build)
precommit: fmt clippy test build
	@echo "All checks passed!"

# Show binary size after release build
size: release
ifeq ($(OS),Windows_NT)
	@dir /s target\release\skrivr.exe 2>nul || echo "Binary not found"
else
	@ls -lh target/release/skrivr 2>/dev/null || echo "Binary not found"
endif

# Build the macOS .app and install it into /Applications, replacing the copy
# already there.
#
# Everyday testing should use `make run-release`, which runs the bare binary
# and is invisible to macOS. This target is the deliberate "make the new build
# my real Skrivr" step, and only this one should touch /Applications.
#
# The app is quit first because replacing a bundle out from under a running
# process leaves it pointing at deleted files, and the old bundle is removed
# rather than copied over because `cp -R` onto an existing bundle merges into
# it — files dropped in the new version would linger as stale resources.
#
# `.metadata_never_index` keeps Spotlight, and so LaunchServices, from
# discovering the bundle left behind in `target/`; that dev bundle carries the
# same `se.kotori.skrivr` identifier as the installed one and would otherwise
# compete with it for the .md/.json/.yaml/.toml associations. `cargo bundle`
# can recreate `target/`, so it is restored here.
install-macos:
ifeq ($(OS),Windows_NT)
	@echo "install-macos is macOS-only"
else
	@osascript -e 'quit app "Kotori Skrivr"' 2>/dev/null || true
	cargo bundle --release
	rm -rf "/Applications/Kotori Skrivr.app"
	cp -R "target/release/bundle/osx/Kotori Skrivr.app" /Applications/
	@touch target/.metadata_never_index
	@echo "Installed Kotori Skrivr $$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' '/Applications/Kotori Skrivr.app/Contents/Info.plist')"
endif

# Help
help:
	@echo "Kotori Skrivr Build Targets:"
	@echo "  make build      - Debug build"
	@echo "  make release    - Release build (optimized)"
	@echo "  make run        - Run debug build"
	@echo "  make run-release- Run release build"
	@echo "  make check      - Check for errors"
	@echo "  make fmt        - Format code"
	@echo "  make clippy     - Run lints"
	@echo "  make test       - Run tests"
	@echo "  make clean      - Clean build artifacts"
	@echo "  make lint       - Format + clippy"
	@echo "  make precommit  - Full pre-commit check"
	@echo "  make size       - Show release binary size"
	@echo "  make install-macos - Build the .app and install it to /Applications"
