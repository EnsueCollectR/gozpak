//! Path helpers mirroring the shell version's directory layout
//! (`create_tmp_dirs`, `repo_read_config`).
//!
//! M1 is read-only: none of these create directories.

use std::path::{Path, PathBuf};

/// System package database: `$GOZPAK_ROOT/var/db/gozpak/installed`.
pub fn installed_db_root(root: &Path) -> PathBuf {
    root.join("var/db/gozpak/installed")
}

/// Repository index cache (after `gozpak sync`): `<cache>/gozpak/repos`.
pub fn repo_cache_dir() -> PathBuf {
    cache_base().join("repos")
}

/// Downloaded source cache: `<cache>/gozpak/sources`.
pub fn source_cache_dir() -> PathBuf {
    cache_base().join("sources")
}

/// Binary tarball cache: `<cache>/gozpak/bin`.
pub fn bin_cache_dir() -> PathBuf {
    cache_base().join("bin")
}

/// XDG cache base: `$XDG_CACHE_HOME/gozpak` or `~/.cache/gozpak`.
fn cache_base() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("gozpak");
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/gozpak")
}

/// User repo URL config: `$GOZPAK_ROOT/etc/gozpak/repos.conf`
/// (one URL per line, `#` comments; the `GOZPAK_REPOS` env var takes
/// priority over this file — see `gozpak-repo::urls`).
pub fn repos_conf(root: &Path) -> PathBuf {
    root.join("etc/gozpak/repos.conf")
}

// ---------------------------------------------------------------------------
// Recipe file helpers (legacy 3-file layout + new TOML layout).
// ---------------------------------------------------------------------------

/// New primary format: a single `recipe.toml` in the recipe directory.
pub fn recipe_toml_file(dir: &Path) -> PathBuf {
    dir.join("recipe.toml")
}

/// Legacy (kiss-style) recipe files.
pub fn recipe_version_file(dir: &Path) -> PathBuf {
    dir.join("version")
}

pub fn recipe_build_file(dir: &Path) -> PathBuf {
    dir.join("build")
}

pub fn recipe_depends_file(dir: &Path) -> PathBuf {
    dir.join("depends")
}

pub fn recipe_sources_file(dir: &Path) -> PathBuf {
    dir.join("sources")
}

pub fn recipe_meta_file(dir: &Path) -> PathBuf {
    dir.join("meta")
}

pub fn recipe_checksums_file(dir: &Path) -> PathBuf {
    dir.join("checksums")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_db_root_layout() {
        assert_eq!(
            installed_db_root(Path::new("/")),
            PathBuf::from("/var/db/gozpak/installed")
        );
        assert_eq!(
            repos_conf(Path::new("/chroot")),
            PathBuf::from("/chroot/etc/gozpak/repos.conf")
        );
    }

    #[test]
    fn recipe_file_names() {
        let dir = Path::new("/src/mesa");
        assert_eq!(recipe_toml_file(dir), PathBuf::from("/src/mesa/recipe.toml"));
        assert_eq!(recipe_version_file(dir), PathBuf::from("/src/mesa/version"));
        assert_eq!(recipe_checksums_file(dir), PathBuf::from("/src/mesa/checksums"));
    }
}
