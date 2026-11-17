//! `gozpak-core`: shared types for the gozpak Rust rewrite (M1).
//!
//! Mirrors the data-side of the shell `gozpak` script:
//!
//! - [`VersionRel`] — `<version> <release>` pairs from `version` files
//! - [`VersionKey`] / [`vercmp`] — v1.1 pacman-style version ordering
//! - [`Pkg`] / [`DepReq`] — installed-package and dependency records
//! - [`InstalledDb`] — the `$GOZPAK_ROOT/var/db/gozpak/installed` directory DB
//! - [`paths`] — recipe-file and cache-directory helpers
//! - [`sha256`] — FIPS 180-4 SHA-256 (no external crates)

pub mod installed_db;
pub mod paths;
pub mod pkg;
pub mod sha256;
pub mod version;

pub use installed_db::InstalledDb;
pub use pkg::{DepReq, Op, Pkg};
pub use version::{VersionKey, VersionRel, pkgver_cmp, vercmp};
