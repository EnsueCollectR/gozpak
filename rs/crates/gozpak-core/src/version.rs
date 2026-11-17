//! Package version + release, as stored in `version` files.

use std::fmt;

/// A package version with its release number, e.g. version `1.3.1`, release `1`.
///
/// `version` files hold a single line `<version> <release>`
/// (see `docs/recipes.md`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionRel {
    pub version: String,
    pub release: String,
}

impl VersionRel {
    pub fn new(version: impl Into<String>, release: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            release: release.into(),
        }
    }

    /// Parse the single-line `<version> <release>` form.
    ///
    /// Exactly two non-empty fields (the shell version dies when the release
    /// field is missing; trailing fields make the line malformed).
    pub fn parse(line: &str) -> Option<Self> {
        let mut parts = line.trim().split_whitespace();
        let version = parts.next()?.to_string();
        let release = parts.next()?.to_string();
        if parts.next().is_some() {
            return None; // not "<version> <release>"
        }
        if version.is_empty() || release.is_empty() {
            return None;
        }
        Some(Self { version, release })
    }

    /// Split the version string into `(major, minor, patch, ident)` on the
    /// `.-+_` character set, mirroring the shell
    /// `IFS=.+-_ read -r repo_major repo_minor repo_patch repo_ident`.
    /// Missing trailing components are empty strings.
    pub fn components(&self) -> (String, String, String, String) {
        let mut it = self
            .version
            .split(|c: char| matches!(c, '.' | '+' | '-' | '_'));
        (
            it.next().unwrap_or("").to_string(),
            it.next().unwrap_or("").to_string(),
            it.next().unwrap_or("").to_string(),
            it.next().unwrap_or("").to_string(),
        )
    }

    /// The `version-release` display form used throughout the CLI output.
    pub fn display(&self) -> String {
        format!("{}-{}", self.version, self.release)
    }
}

impl fmt::Display for VersionRel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

// ---------------------------------------------------------------------------
// v1.1 version comparison (pacman `vercmp` semantics)
// ---------------------------------------------------------------------------

/// The three keys the solver and the repo index compare in order:
/// epoch (numeric) → version (pacman chunk compare) → release (numeric).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct VersionKey {
    pub epoch: u64,
    pub version: String,
    pub release: String,
}

impl VersionKey {
    pub fn new(epoch: u64, version: impl Into<String>, release: impl Into<String>) -> Self {
        Self {
            epoch,
            version: version.into(),
            release: release.into(),
        }
    }
}

/// Pacman-style comparison of two versions.
///
/// `epoch` is compared numerically first; then the version core with
/// pacman chunk rules (numeric chunks vs alphabetic chunks; a release
/// tag such as `rc`/`a`/`b`/`alpha` sorts BELOW the plain release of
/// that number, e.g. `3.5.1rc1 < 3.5.1`); finally the release number,
/// numerically.
pub fn vercmp(a: &VersionKey, b: &VersionKey) -> std::cmp::Ordering {
    let o = a.epoch.cmp(&b.epoch);
    if o != std::cmp::Ordering::Equal {
        return o;
    }
    let o = pkgver_cmp(&a.version, &b.version);
    if o != std::cmp::Ordering::Equal {
        return o;
    }
    release_cmp(&a.release, &b.release)
}

/// Numeric release compare with a string fallback for non-numeric values.
fn release_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<u64>(), b.parse::<u64>()) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

/// Pacman `pkgver` compare: alternating alphabetic / numeric blocks.
///
/// - Alphabetic runs compare byte-wise; when one side has an alphabetic
///   run and the other side is at a digit or the end, the side WITH the
///   letters is OLDER (release tags sort below the release).
/// - Numeric runs compare numerically.
/// - At the end, the side that ran out of blocks while the other still
///   has a *numeric* remainder is older; a remaining alphabetic
///   remainder makes the shorter side newer (covered by the alphabetic
///   rule above).
pub fn pkgver_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let mut sa = a.as_bytes();
    let mut sb = b.as_bytes();
    loop {
        // Leading non-digit run.
        let (ta, na) = take_run(&mut sa, |c| !c.is_ascii_digit());
        let (tb, nb) = take_run(&mut sb, |c| !c.is_ascii_digit());
        match (ta.is_empty(), tb.is_empty()) {
            (false, false) => {
                let o = ta.cmp(tb);
                if o != Ordering::Equal {
                    return o;
                }
            }
            (false, true) => return Ordering::Less,
            (true, false) => return Ordering::Greater,
            (true, true) => {}
        }
        sa = na;
        sb = nb;
        // Leading digit run.
        let (da, na) = take_digits(sa);
        let (db, nb) = take_digits(sb);
        match (da.is_none(), db.is_none()) {
            (false, false) => {
                let o = da.unwrap().cmp(&db.unwrap());
                if o != Ordering::Equal {
                    return o;
                }
            }
            (false, true) => return Ordering::Greater,
            (true, false) => return Ordering::Less,
            (true, true) => {}
        }
        sa = na;
        sb = nb;
        if sa.is_empty() && sb.is_empty() {
            return Ordering::Equal;
        }
    }
}

fn take_run<'a>(
    bytes: &mut &'a [u8],
    pred: impl Fn(u8) -> bool,
) -> (&'a [u8], &'a [u8]) {
    let mut end = 0;
    while end < bytes.len() && pred(bytes[end]) {
        end += 1;
    }
    let run = &bytes[..end];
    *bytes = &bytes[end..];
    (run, *bytes)
}

/// Leading digit run as a (saturating) u64.
fn take_digits<'a>(bytes: &'a [u8]) -> (Option<u64>, &'a [u8]) {
    let mut n: u64 = 0;
    let mut any = false;
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        n = n.saturating_mul(10).saturating_add(u64::from(bytes[end] - b'0'));
        any = true;
        end += 1;
    }
    (any.then_some(n), &bytes[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_release() {
        let v = VersionRel::parse("1.3.1 1").unwrap();
        assert_eq!(v.version, "1.3.1");
        assert_eq!(v.release, "1");
        assert_eq!(v.display(), "1.3.1-1");
    }

    #[test]
    fn parse_tolerates_extra_whitespace_and_trailing_newline() {
        let v = VersionRel::parse("  2.0.0\t2\n").unwrap();
        assert_eq!(v.display(), "2.0.0-2");
    }

    #[test]
    fn parse_requires_release_field() {
        assert!(VersionRel::parse("1.0").is_none());
        assert!(VersionRel::parse("  ").is_none());
        assert!(VersionRel::parse("").is_none());
        // Three fields is not "<version> <release>".
        assert!(VersionRel::parse("not a version").is_none());
        assert!(VersionRel::parse("1.0 1 x").is_none());
    }

    #[test]
    fn components_split_on_dot_dash() {
        let v = VersionRel::new("1.3.1-1", "1");
        assert_eq!(v.components(), ("1".into(), "3".into(), "1".into(), "1".into()));

        let v = VersionRel::new("24.0", "1");
        assert_eq!(v.components(), ("24".into(), "0".into(), String::new(), String::new()));

        let v = VersionRel::new("1.2+git", "1");
        assert_eq!(v.components(), ("1".into(), "2".into(), "git".into(), String::new()));
    }

    // -----------------------------------------------------------------------
    // vercmp (v1.1)
    // -----------------------------------------------------------------------

    fn key(epoch: u64, version: &str, release: &str) -> VersionKey {
        VersionKey::new(epoch, version, release)
    }

    #[test]
    fn release_tags_sort_below_release() {
        use std::cmp::Ordering;
        assert_eq!(
            pkgver_cmp("3.5.1rc1", "3.5.1"),
            Ordering::Less,
            "3.5.1rc1 < 3.5.1"
        );
        assert_eq!(pkgver_cmp("3.5.1", "3.5.1rc1"), Ordering::Greater);
        assert_eq!(pkgver_cmp("1.0a1", "1.0"), Ordering::Less);
        assert_eq!(pkgver_cmp("1.0b2", "1.0rc1"), Ordering::Less);
        assert_eq!(pkgver_cmp("1.0alpha", "1.0beta"), Ordering::Less);
        assert_eq!(pkgver_cmp("3.5.1", "3.5.1"), Ordering::Equal);
    }

    #[test]
    fn numeric_chunks_compare_numerically() {
        use std::cmp::Ordering;
        assert_eq!(pkgver_cmp("1.9", "1.10"), Ordering::Less);
        assert_eq!(pkgver_cmp("1.0a2", "1.0a10"), Ordering::Less);
        assert_eq!(pkgver_cmp("2.0", "1.99"), Ordering::Greater);
    }

    #[test]
    fn epoch_beats_version() {
        use std::cmp::Ordering;
        // 1.0-1 with epoch 1 beats 0.9-2 with epoch 0.
        assert_eq!(
            vercmp(&key(1, "1.0", "1"), &key(0, "0.9", "2")),
            Ordering::Greater
        );
        assert_eq!(
            vercmp(&key(0, "0.9", "2"), &key(1, "1.0", "1")),
            Ordering::Less
        );
        // Equal epochs fall through to the version compare.
        assert_eq!(
            vercmp(&key(2, "0.9", "9"), &key(2, "1.0", "1")),
            Ordering::Less
        );
    }

    #[test]
    fn release_is_the_tiebreak() {
        use std::cmp::Ordering;
        assert_eq!(
            vercmp(&key(0, "3.5.1", "2"), &key(0, "3.5.1", "1")),
            Ordering::Greater
        );
        // Numeric, not lexical: release 10 > release 9.
        assert_eq!(
            vercmp(&key(0, "3.5.1", "10"), &key(0, "3.5.1", "9")),
            Ordering::Greater
        );
    }
}
