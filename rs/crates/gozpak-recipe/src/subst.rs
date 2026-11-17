//! Source-line variable substitution and local resolution.
//!
//! Two substitution dialects, mirroring the shell version:
//!
//! - **Shell style** (legacy `sources` files): bare tokens `VERSION`,
//!   `RELEASE`, `MAJOR`, `MINOR`, `PATCH`, `IDENT`, `PACKAGE` are replaced
//!   everywhere they occur. A backslash-escaped marker (`\VERSION`) is
//!   turned into the *literal marker word* with the backslash dropped —
//!   this is exactly the `fnr` table used by `pkg_source_resolve`: for each
//!   marker `\M`→` `, `M`→value, ` `→`M` (the placeholder works because
//!   substitution is applied to whitespace-split source tokens).
//! - **Dollar style** (new TOML `[source].url`): `$VERSION`, `$RELEASE`,
//!   `$MAJOR`, `$MINOR`, `$PATCH`, `$PACKAGE` (plus `$IDENT` for shell
//!   parity); `\$` escapes the dollar.

use gozpak_core::VersionRel;

/// Substitution context derived from a recipe's name + version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subst {
    pub package: String,
    pub version: String,
    pub release: String,
    pub major: String,
    pub minor: String,
    pub patch: String,
    pub ident: String,
    /// v1.1: `[package].epoch` (0 for legacy recipes).
    pub epoch: u64,
}

impl Subst {
    pub fn from_recipe(name: &str, version: &VersionRel, epoch: u64) -> Self {
        let (major, minor, patch, ident) = version.components();
        Self {
            package: name.to_string(),
            version: version.version.clone(),
            release: version.release.clone(),
            major,
            minor,
            patch,
            ident,
            epoch,
        }
    }

    /// Shell-style substitution (legacy `sources` lines).
    ///
    /// Mirrors the `fnr` call chain in `pkg_source_resolve`: substitution is
    /// applied to each whitespace-separated token, and for every marker M
    /// (in table order) the token gets `\M`→` `, `M`→value, ` `→`M`.
    /// The space placeholder is safe because tokens contain no whitespace,
    /// and it is what makes `\M` survive as the literal word `M`.
    pub fn apply_shell(&self, s: &str) -> String {
        let markers: [(&str, &str); 7] = [
            ("VERSION", &self.version),
            ("RELEASE", &self.release),
            ("MAJOR", &self.major),
            ("MINOR", &self.minor),
            ("PATCH", &self.patch),
            ("IDENT", &self.ident),
            ("PACKAGE", &self.package),
        ];
        s.split_whitespace()
            .map(|token| {
                let mut t = token.to_string();
                for &(marker, value) in &markers {
                    let esc = format!("\\{marker}");
                    t = t.replace(&esc, " ");
                    t = t.replace(marker, value);
                    t = t.replace(' ', marker);
                }
                t
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Dollar-style substitution (TOML `[source].url`).
    ///
    /// `$VERSION` etc. are replaced; `\$` yields a literal `$`; an unknown
    /// `$NAME` is left verbatim.
    pub fn apply(&self, s: &str) -> String {
        let epoch = self.epoch.to_string();
        let table: [(&str, &str); 8] = [
            ("VERSION", &self.version),
            ("RELEASE", &self.release),
            ("MAJOR", &self.major),
            ("MINOR", &self.minor),
            ("PATCH", &self.patch),
            ("IDENT", &self.ident),
            ("PACKAGE", &self.package),
            ("EPOCH", &epoch),
        ];

        let chars: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len());
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '\\' if i + 1 < chars.len() && chars[i + 1] == '$' => {
                    out.push('$');
                    i += 2;
                }
                '$' => {
                    // Read the variable name.
                    let mut name = String::new();
                    let mut j = i + 1;
                    while j < chars.len() && chars[j].is_ascii_uppercase() {
                        name.push(chars[j]);
                        j += 1;
                    }
                    if name.is_empty() {
                        out.push('$');
                        i += 1;
                        continue;
                    }
                    if let Some((_, value)) = table.iter().find(|(k, _)| **k == name) {
                        out.push_str(value);
                    } else {
                        out.push('$');
                        out.push_str(&name);
                    }
                    i = j;
                }
                c => {
                    out.push(c);
                    i += 1;
                }
            }
        }
        out
    }
}

/// The result of trying to locate a source locally (M1 has no network).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// A file that exists on disk (local path or cached download) —
    /// hashable for the `checksum` command.
    File(std::path::PathBuf),
    /// Not available locally: a remote source without a cached copy, or a
    /// git checkout (the shell version never checksums git sources).
    Unavailable,
}

/// Where a cached remote download of `package` lands
/// (`<cache>/<pkg>/<dest?>/ <last url segment>`), mirroring
/// `pkg_source_resolve`'s `_des` computation.
pub fn cache_path_for(package: &str, source_cache: &std::path::Path, url: &str, dest: Option<&str>) -> std::path::PathBuf {
    let base = url.rsplit('/').next().unwrap_or(url);
    let base = base.split(['?', '#']).next().unwrap_or(base).to_string();
    let mut p = source_cache.join(package);
    if let Some(d) = dest {
        p = p.join(d);
    }
    p.join(base)
}

/// Resolve one normalized source to a local file, if one exists.
///
/// - Local sources resolve against `recipe_dir` (absolute paths as-is).
/// - Remote sources resolve against the download cache (M1 is read-only:
///   only previously downloaded files are visible; M2 adds the download).
/// - Git sources are never resolvable in M1.
pub fn resolve_local(
    src: &crate::Source,
    recipe_dir: &std::path::Path,
    source_cache: &std::path::Path,
    package: &str,
) -> Resolved {
    match src {
        crate::Source::Local { path } => {
            let p = if path.is_absolute() {
                path.to_path_buf()
            } else {
                recipe_dir.join(path)
            };
            if p.exists() {
                Resolved::File(p)
            } else {
                Resolved::Unavailable
            }
        }
        crate::Source::Files { files } => {
            // M1: only previously cached downloads are visible (no network).
            for f in files {
                let cached = cache_path_for(package, source_cache, &f.url, f.dest.as_deref());
                if cached.is_file() {
                    return Resolved::File(cached);
                }
            }
            Resolved::Unavailable
        }
        crate::Source::Git { .. } => Resolved::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Subst {
        Subst::from_recipe(
            "acl",
            &VersionRel::new("2.3.2", "1"),
            0,
        )
    }

    #[test]
    fn shell_style_bare_tokens() {
        let s = ctx();
        assert_eq!(s.apply_shell("acl-VERSION.tar.xz"), "acl-2.3.2.tar.xz");
        // fnr semantics: bare markers are replaced even inside ${...}.
        assert_eq!(s.apply_shell("rel-RELEASE-${MAJOR}"), "rel-1-${2}");
    }

    #[test]
    fn shell_style_components() {
        let s = Subst::from_recipe(
            "mesa",
            &VersionRel::new("24.0.3+git", "2"),
            0,
        );
        assert_eq!(
            s.apply_shell("MAJOR.MINOR.PATCH PACKAGE RELEASE IDENT"),
            "24.0.3 mesa 2 git"
        );
    }

    #[test]
    fn shell_style_escaped_marker() {
        let s = ctx();
        // \VERSION drops the backslash and keeps the literal marker word
        // (fnr: \M -> ' ', M -> value, ' ' -> M).
        assert_eq!(s.apply_shell(r"a\VERSIONb"), "aVERSIONb");
    }

    #[test]
    fn dollar_style() {
        let s = ctx();
        assert_eq!(s.apply("acl-$VERSION.tar.xz"), "acl-2.3.2.tar.xz");
        assert_eq!(s.apply(r"escaped \$VERSION literal"), "escaped $VERSION literal");
        assert_eq!(s.apply("$UNKNOWN stays"), "$UNKNOWN stays");
        assert_eq!(s.apply("$"), "$");
        // v1.1: $EPOCH.
        assert_eq!(s.apply("e$EPOCH-$VERSION"), "e0-2.3.2");
        let s = Subst::from_recipe("openssl", &VersionRel::new("3.5.1", "1"), 2);
        assert_eq!(s.apply("epoch-$EPOCH rel-$RELEASE"), "epoch-2 rel-1");
    }

    #[test]
    fn cache_path_for_strips_query_and_uses_dest() {
        let cache = std::path::Path::new("/cache/gozpak/sources");
        let p = cache_path_for(
            "acl",
            cache,
            "https://host/acl-2.3.2.tar.xz?token=1",
            None,
        );
        assert_eq!(
            p,
            std::path::PathBuf::from("/cache/gozpak/sources/acl/acl-2.3.2.tar.xz")
        );
        let p = cache_path_for("acl", cache, "https://host/x/acl.tar.xz", Some("src"));
        assert_eq!(
            p,
            std::path::PathBuf::from("/cache/gozpak/sources/acl/src/acl.tar.xz")
        );
    }
}
