//! The installed-packages database.
//!
//! Layout (identical to the shell version, see `create_tmp_dirs`):
//!
//! ```text
//! $GOZPAK_ROOT/var/db/gozpak/installed/
//!     <pkg>/version      # "<version> <release>"
//!     <pkg>/manifest     # (M2: file list; not read in M1)
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Pkg, VersionRel};

/// Read-only handle to the installed DB (M1 never writes it).
#[derive(Debug, Clone)]
pub struct InstalledDb {
    root: PathBuf,
}

impl InstalledDb {
    /// `root` is the *installed* directory itself
    /// (`$GOZPAK_ROOT/var/db/gozpak/installed`).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn pkg_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// A package counts as installed when its directory exists
    /// (same test the shell version uses in `repo_get`).
    pub fn is_installed(&self, name: &str) -> bool {
        self.pkg_dir(name).is_dir()
    }

    /// Read `<root>/<name>/version` → `<version> <release>`.
    pub fn version_of(&self, name: &str) -> Option<VersionRel> {
        let text = fs::read_to_string(self.pkg_dir(name).join("version")).ok()?;
        VersionRel::parse(&text)
    }

    /// All installed packages, sorted by name.
    ///
    /// A missing/unreadable DB root yields an empty list (the shell version's
    /// glob would expand to nothing on a system with no packages).
    /// Malformed entries are skipped rather than aborting the whole listing.
    pub fn list(&self) -> Vec<Pkg> {
        let entries = match fs::read_dir(&self.root) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };
        let mut pkgs = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let line = fs::read_to_string(path.join("version")).ok();
            if let Some(pkg) = Pkg::from_db_entry(&name, line.as_deref()) {
                pkgs.push(pkg);
            }
        }
        pkgs.sort_by(|a, b| a.name.cmp(&b.name));
        pkgs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_tmp(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("gozpak-core-test-{}-{}-{}", std::process::id(), name, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(pkg_root: &Path, pkg: &str, version_line: &str) {
        let dir = pkg_root.join(pkg);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("version"), version_line).unwrap();
    }

    #[test]
    fn missing_root_lists_nothing() {
        let db = InstalledDb::new(unique_tmp("missing-root").join("does-not-exist"));
        assert!(db.list().is_empty());
        assert!(db.version_of("zlib").is_none());
        assert!(!db.is_installed("zlib"));
    }

    #[test]
    fn lists_packages_sorted_with_versions() {
        let root = unique_tmp("list");
        write(&root, "zlib", "1.3.1 1");
        write(&root, "hello", "2.0 4");
        write(&root, "broken", "not a version"); // malformed → skipped
        fs::create_dir_all(root.join("notadir")).unwrap();
        fs::write(root.join("stray-file"), "x").unwrap();

        let db = InstalledDb::new(&root);
        let pkgs = db.list();
        let got: Vec<_> =
            pkgs.iter().map(|p| (p.name.clone(), p.version.display())).collect();
        assert_eq!(
            got,
            vec![
                ("hello".to_string(), "2.0-4".to_string()),
                ("zlib".to_string(), "1.3.1-1".to_string()),
            ]
        );
        assert!(db.is_installed("zlib"));
        assert!(!db.is_installed("nope"));
        assert_eq!(db.version_of("zlib").unwrap().display(), "1.3.1-1");
        let _ = fs::remove_dir_all(&root);
    }
}
