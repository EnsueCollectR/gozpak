//! `repo.index` parse/serialize (v1.2).
//!
//! v1.2 line format — 11 tab-separated columns:
//!
//! ```text
//! name \t version \t release \t epoch \t arch \t sha256 \t url \t depends \t provides \t conflicts
//! ```
//!
//! `depends` / `provides` / `conflicts` are space-free lists: entries are
//! v1.1 constraint strings (`zlib>=1.3`, `libssl=3`, ... — no spaces) and
//! multiple entries are joined with `|`; an empty list is `-`.
//! The raw strings round-trip verbatim (parsing into `DepReq` happens
//! where the solver consumes them — not here).
//!
//! **Backward tolerance:** lines in the old 7-field reference format
//! (`name version release sha256 size depends description`, see
//! contrib/gozpak-repo-index) still parse: `epoch` defaults to 0,
//! `arch` to `x86_64`, `url` stays empty, and the dependency lists come
//! out empty (`-`). Lines with 8–10 fields are malformed (only 7-field
//! legacy and 11-field v1.2 exist).

use std::fs;
use std::path::Path;

/// One package entry in a `repo.index` file (v1.2: 11 columns).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndexEntry {
    pub name: String,
    pub version: String,
    pub release: u32,
    /// v1.1: default 0.
    pub epoch: u64,
    /// v1.1: default "x86_64".
    pub arch: String,
    pub sha256: String,
    /// Where the binary package lives (URL or path).
    pub url: String,
    /// Space-free dependency list: constraint strings joined with `|`;
    /// `-` when empty. Carried verbatim (round-trip safe).
    pub depends: String,
    /// Virtual packages this name provides (same list syntax).
    pub provides: String,
    /// Conflicting packages (same list syntax).
    pub conflicts: String,
}

fn dep_list_or_dash(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        "-".to_string()
    } else {
        t.to_string()
    }
}

/// Split a v1.2 `|`-joined dependency list into its verbatim entries.
/// `-` or empty → no entries (an empty list).
pub fn split_dep_list(list: &str) -> Vec<&str> {
    let t = list.trim();
    if t.is_empty() || t == "-" {
        return Vec::new();
    }
    t.split('|').map(str::trim).filter(|s| !s.is_empty()).collect()
}

impl IndexEntry {
    /// Parse one `repo.index` line. Returns `None` for blank lines, lines
    /// without at least a name and version, and malformed field counts.
    ///
    /// - 11 fields → v1.2 (`... epoch arch sha256 url depends provides
    ///   conflicts`); missing numeric fields default (release/epoch → 0,
    ///   arch → "x86_64", dep lists → `-`).
    /// - 7 fields → legacy reference format (`... sha256 size depends
    ///   description`): epoch 0, arch "x86_64", empty url, empty deps.
    /// - 2–6 fields → v1.2 with omitted trailing columns (defaults).
    /// - 8–10 fields → malformed.
    pub fn parse_line(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split('\t').collect();
        let n = fields.len();

        let name = fields[0].trim().to_string();
        let version = fields.get(1).copied().unwrap_or("").trim().to_string();
        if name.is_empty() || version.is_empty() {
            return None;
        }
        let num = |i: usize, default: u64| -> u64 {
            fields
                .get(i)
                .map(|f| f.trim().parse().unwrap_or(default))
                .unwrap_or(default)
        };

        if n == 7 {
            // Legacy 7-field reference format:
            // name version release sha256 size depends description
            return Some(Self {
                name,
                version,
                release: num(2, 0) as u32,
                epoch: 0,
                arch: "x86_64".into(),
                sha256: fields[3].trim().to_string(),
                url: String::new(),
                depends: "-".into(),
                provides: "-".into(),
                conflicts: "-".into(),
            });
        }
        // v1.2: 2–6 fields = trailing columns omitted (defaults); legacy = 7;
        // full = 10. 8–9 or 11+ fields correspond to no defined format.
        if !(2..=6).contains(&n) && n != 10 {
            return None;
        }
        Some(Self {
            name,
            version,
            release: num(2, 0) as u32,
            epoch: num(3, 0),
            arch: fields
                .get(4)
                .map(|f| f.trim())
                .filter(|f| !f.is_empty())
                .unwrap_or("x86_64")
                .to_string(),
            sha256: fields.get(5).map(|f| f.trim().to_string()).unwrap_or_default(),
            url: fields.get(6).map(|f| f.trim().to_string()).unwrap_or_default(),
            depends: dep_list_or_dash(fields.get(7).map(|f| f.trim()).unwrap_or("-")),
            provides: dep_list_or_dash(fields.get(8).map(|f| f.trim()).unwrap_or("-")),
            conflicts: dep_list_or_dash(fields.get(9).map(|f| f.trim()).unwrap_or("-")),
        })
    }

    /// Render as a v1.2 tab-separated index line (10 columns).
    pub fn to_line(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.name,
            self.version,
            self.release,
            self.epoch,
            self.arch,
            self.sha256,
            self.url,
            dep_list_or_dash(&self.depends),
            dep_list_or_dash(&self.provides),
            dep_list_or_dash(&self.conflicts)
        )
    }

    /// The `|`-joined `depends` list, verbatim (empty list → `-`).
    pub fn depends_list(&self) -> Vec<&str> {
        split_dep_list(&self.depends)
    }
    pub fn provides_list(&self) -> Vec<&str> {
        split_dep_list(&self.provides)
    }
    pub fn conflicts_list(&self) -> Vec<&str> {
        split_dep_list(&self.conflicts)
    }
}

/// A parsed `repo.index` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepoIndex {
    pub entries: Vec<IndexEntry>,
}

/// Errors from [`RepoIndex::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoIndexError {
    /// Line (1-based) that could not be parsed.
    Malformed { line_number: usize, line: String },
    /// I/O or read error (line_number 0).
    Io { line: String },
}

impl std::fmt::Display for RepoIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoIndexError::Malformed {
                line_number,
                line,
            } => write!(f, "malformed repo.index line {line_number}: '{line}'"),
            RepoIndexError::Io { line } => write!(f, "{line}"),
        }
    }
}

impl std::error::Error for RepoIndexError {}

impl RepoIndex {
    /// Parse `repo.index` text. Blank lines and `#` comments are skipped;
    /// any other line that fails to parse is an error (the shell reader
    /// is fully lenient; strictness is intentional so broken mirrors are
    /// caught at sync time in M2).
    pub fn parse(text: &str) -> Result<Self, RepoIndexError> {
        let mut entries = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            match IndexEntry::parse_line(line) {
                Some(e) => entries.push(e),
                None => {
                    return Err(RepoIndexError::Malformed {
                        line_number: n + 1,
                        line: line.to_string(),
                    })
                }
            }
        }
        Ok(Self { entries })
    }

    /// Serialize as TSV (entries in order, trailing newline).
    pub fn to_tsv(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        for e in &self.entries {
            out.push_str(&e.to_line());
            out.push('\n');
        }
        out
    }

    pub fn load(path: &Path) -> Result<Self, RepoIndexError> {
        let text = fs::read_to_string(path).map_err(|e| RepoIndexError::Io {
            line: format!("cannot read {}: {e}", path.display()),
        })?;
        Self::parse(&text)
    }

    /// Write TSV to `path`, overwriting.
    pub fn write(&self, path: &Path) -> Result<(), RepoIndexError> {
        fs::write(path, self.to_tsv()).map_err(|e| RepoIndexError::Io {
            line: format!("cannot write {}: {e}", path.display()),
        })
    }

    /// Find an entry by package name (first match).
    pub fn find(&self, name: &str) -> Option<&IndexEntry> {
        self.entries.iter().find(|e| e.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v12_full_line() {
        let e = IndexEntry::parse_line(
            "curl\t8.7.1\t1\t0\tx86_64\tsha1\tu1\topenssl>=1.3|pcre2\tlibcurl=8\t-",
        )
        .unwrap();
        assert_eq!(e.name, "curl");
        assert_eq!(e.version, "8.7.1");
        assert_eq!(e.release, 1);
        assert_eq!(e.epoch, 0);
        assert_eq!(e.arch, "x86_64");
        assert_eq!(e.sha256, "sha1");
        assert_eq!(e.url, "u1");
        assert_eq!(e.depends_list(), vec!["openssl>=1.3", "pcre2"]);
        assert_eq!(e.provides_list(), vec!["libcurl=8"]);
        assert_eq!(e.conflicts_list(), Vec::<&str>::new());
    }

    #[test]
    fn legacy_7_field_line_parses() {
        // Old reference format: name version release sha256 size depends description
        let e = IndexEntry::parse_line(
            "curl\t8.7.1\t1\ta1b2c3d4\t1234567\topenssl,zlib\tCommand-line tool",
        )
        .unwrap();
        assert_eq!(e.epoch, 0);
        assert_eq!(e.arch, "x86_64");
        assert_eq!(e.sha256, "a1b2c3d4");
        assert!(e.url.is_empty());
        assert_eq!(e.depends, "-");
        assert_eq!(e.provides, "-");
        assert_eq!(e.conflicts, "-");
    }

    #[test]
    fn parse_lenient_missing_trailing() {
        let e = IndexEntry::parse_line("zlib\t1.3.1").unwrap();
        assert_eq!(e.release, 0);
        assert_eq!(e.epoch, 0);
        assert_eq!(e.arch, "x86_64");
        assert!(e.sha256.is_empty());
        assert!(e.url.is_empty());
        assert_eq!(e.depends, "-");

        // No name/version → None.
        assert!(IndexEntry::parse_line("\t\t").is_none());
        assert!(IndexEntry::parse_line("").is_none());
        assert!(IndexEntry::parse_line("just-a-name").is_none());
    }

    #[test]
    fn malformed_field_counts_rejected() {
        // 8–9 fields do not correspond to any defined format.
        assert!(IndexEntry::parse_line("a\t1\t0\t0\tx86_64\tsha\tu\tx").is_none());
        assert!(IndexEntry::parse_line(
            "a\t1\t0\t0\tx86_64\tsha\tu\tx\t-y"
        )
        .is_none());
        // 11-field line: full format is 10 columns, so 11+ is malformed.
        assert!(IndexEntry::parse_line(
            "a\t1\t0\t0\tx86_64\tsha\tu\td\tpr\tc\txtra"
        )
        .is_none());
    }

    #[test]
    fn constraint_strings_roundtrip_verbatim() {
        let line = "openssl\t3.5.1\t1\t1\tx86_64\tsha2\thttps://h/o.gpkg\tzlib>=1.3|pcre2\tlibssl=3\t-";
        let e = IndexEntry::parse_line(line).unwrap();
        assert_eq!(e.to_line(), line, "v1.2 line must round-trip byte-identical");
    }

    #[test]
    fn parse_roundtrip_tsv() {
        let text = "curl\t8.7.1\t1\t0\tx86_64\tsha1\tu1\ta>=1|b\tc=2\t-\n\
                    openssl\t3.5.1\t1\t1\tx86_64\tsha2\tu2\tzlib>=1.3\t-\t-\n";
        let idx = RepoIndex::parse(text).unwrap();
        assert_eq!(idx.entries.len(), 2);
        assert_eq!(idx.find("openssl").unwrap().epoch, 1);

        let out = idx.to_tsv();
        assert_eq!(out, text, "roundtrip must be byte-identical");

        // blank lines and comments are skipped
        let idx = RepoIndex::parse("\n# header comment\nzlib\t1.3.1\n").unwrap();
        assert_eq!(idx.entries.len(), 1);
    }

    #[test]
    fn parse_strict_rejects_garbage() {
        // A line with only a name (no version) is malformed → error.
        let err = RepoIndex::parse("just-a-name").unwrap_err();
        match err {
            RepoIndexError::Malformed { line_number, .. } => assert_eq!(line_number, 1),
            _ => panic!("expected Malformed"),
        }
    }

    #[test]
    fn load_and_write_file() {
        let dir =
            std::env::temp_dir().join(format!("gozpak-repo-io-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("repo.index");
        std::fs::write(
            &p,
            "a\t1\t0\t0\tx86_64\tsha\thttps://h/a.tar.xz\t-\t-\t-\n",
        )
        .unwrap();

        let idx = RepoIndex::load(&p).unwrap();
        assert_eq!(idx.find("a").unwrap().url, "https://h/a.tar.xz");
        assert_eq!(idx.find("a").unwrap().depends, "-");

        let p2 = dir.join("repo.index.out");
        idx.write(&p2).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p2).unwrap(),
            "a\t1\t0\t0\tx86_64\tsha\thttps://h/a.tar.xz\t-\t-\t-\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
