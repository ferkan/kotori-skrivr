//! Product identity.
//!
//! Single source of truth for the user-visible product name. This is a fork of
//! [Ferrite](https://github.com/OlaProeis/Ferrite) and the upstream name was
//! left scattered through the UI — the window title, the welcome screen, the
//! about panel and the HTML export generator tag all said "Ferrite", which is
//! the wrong product name to show a user of this application.
//!
//! Attribution to upstream belongs in the about screen and `Cargo.toml`, not in
//! the window title.

/// User-visible product name.
pub const APP_NAME: &str = "Kotori Skrivr";

/// Upstream project this is forked from, for attribution in about/credits.
pub const UPSTREAM_NAME: &str = "Ferrite";
