//! `.gpkg` binary package naming (v1.2).
//!
//! A `.gpkg` is a tar.zst archive (`manifest.toml` + `rootfs/`) whose
//! filename follows the pacman-style convention:
//!
//! ```text
//! <name>-<epoch?>:<version>-<release>.gpkg
//! ```
//!
//! - `openssl-3.5.1-1.gpkg` — epoch 0 (the `epoch:` token is omitted).
//! - `libfoo-1:2.0-1.gpkg` — epoch 1 (the token appears only when > 0).
//!
//! Package names keep their dashes; the `:` makes the epoch unambiguous.
//!
//! **The filename is NEVER parsed back into fields.** The repo index
//! (`repo.index` / `index.json`) and the package's `manifest.toml` are
//! the only sources of truth — a split-on-dash filename parser would lie
//! for any dashed package name. Use [`gpkg_filename`] to *build* names;
//! never write a parser for them.

/// Build the v1.2 `.gpkg` filename for a package.
///
/// The `epoch:` token is included only when `epoch > 0` (the 0 epoch is
/// the default and never written). Do not parse the result back into
/// fields (see the module docs).
pub fn gpkg_filename(name: &str, epoch: u64, version: &str, release: &str) -> String {
    if epoch > 0 {
        format!("{name}-{epoch}:{version}-{release}.gpkg")
    } else {
        format!("{name}-{version}-{release}.gpkg")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_omitted() {
        assert_eq!(gpkg_filename("openssl", 0, "3.5.1", "1"), "openssl-3.5.1-1.gpkg");
    }

    #[test]
    fn epoch_token_present_when_positive() {
        assert_eq!(gpkg_filename("libfoo", 1, "2.0", "1"), "libfoo-1:2.0-1.gpkg");
        assert_eq!(
            gpkg_filename("golang", 12, "1.24.0", "3"),
            "golang-12:1.24.0-3.gpkg"
        );
    }

    #[test]
    fn dashed_names_are_fine_but_never_parsed_back() {
        // Documented: dashed names build fine; nothing in this crate may
        // split the filename back into fields.
        assert_eq!(
            gpkg_filename("nlohmann-json", 0, "3.11.3", "2"),
            "nlohmann-json-3.11.3-2.gpkg"
        );
    }
}
