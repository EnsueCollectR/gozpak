//! The binary-repo index (`index.json`), v1.2.
//!
//! A binary repo (GitHub Releases, gozjaro-bootstrap#94) ships
//! `index.json` alongside an `index.json.sig` (ed25519; the public key is
//! embedded in gozpak). **Signature verification lands in M2** — M1 parses
//! the index only.
//!
//! `index.json` is a JSON array, one object per package:
//!
//! ```json
//! [
//!   {
//!     "name": "openssl",
//!     "version": "3.5.1",
//!     "release": 1,
//!     "epoch": 1,
//!     "arch": "x86_64",
//!     "sha256": "...",
//!     "size": 123456,
//!     "url": "https://.../openssl-1:3.5.1-1.gpkg",
//!     "depends": ["zlib>=1.3", "pcre2"],
//!     "provides": ["libssl=3"],
//!     "conflicts": [],
//!     "replaces": []
//!   }
//! ]
//! ```
//!
//! The index carries the solver's input (depends/provides/conflicts):
//! dependency solving must NOT require downloading `.gpkg` manifests —
//! `manifest.toml` is read at install time only. Dependency entries are
//! verbatim v1.1 constraint strings.

use serde::{Deserialize, Serialize};

/// One package entry of a binary repo's `index.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryEntry {
    pub name: String,
    pub version: String,
    pub release: u32,
    /// v1.1: default 0 when omitted.
    #[serde(default)]
    pub epoch: u64,
    /// v1.1: default "x86_64" when omitted.
    #[serde(default = "default_arch")]
    pub arch: String,
    pub sha256: String,
    /// Tarball size in bytes; default 0 when omitted.
    #[serde(default)]
    pub size: u64,
    pub url: String,
    /// Verbatim v1.1 constraint strings (`zlib>=1.3`, ...).
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub replaces: Vec<String>,
}

fn default_arch() -> String {
    "x86_64".into()
}

/// Errors from [`parse_index_json`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryIndexError {
    /// The text is not a JSON array of package objects.
    Parse {
        message: String,
        /// First problematic JSON key/path, when the error carries one.
        at: Option<String>,
    },
}

impl std::fmt::Display for BinaryIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryIndexError::Parse { message, at } => {
                write!(f, "invalid index.json: {message}")?;
                if let Some(at) = at {
                    write!(f, " (at {at})")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for BinaryIndexError {}

/// Parse `index.json` text into [`BinaryEntry`]s.
///
/// NOTE: M1 does not verify the accompanying `index.json.sig`
/// (ed25519, embedded public key) — signature verification and the
/// optional per-package `.gpkg.sig` checks are M2 work.
pub fn parse_index_json(text: &str) -> Result<Vec<BinaryEntry>, BinaryIndexError> {
    match serde_json::from_str::<Vec<BinaryEntry>>(text) {
        Ok(entries) => Ok(entries),
        Err(e) => {
            let mut at = None;
            let msg = e.to_string();
            if let Some(pos) = msg.find("at line") {
                at = Some(msg[pos..].to_string());
            }
            Err(BinaryIndexError::Parse {
                message: msg.split(" at ").next().unwrap_or(&msg).to_string(),
                at,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_entry() {
        let json = r#"{
            "name": "openssl",
            "version": "3.5.1",
            "release": 1,
            "epoch": 1,
            "arch": "x86_64",
            "sha256": "ab12",
            "size": 4096,
            "url": "https://releases.gozjaro.org/openssl-1:3.5.1-1.gpkg",
            "depends": ["zlib>=1.3", "pcre2"],
            "provides": ["libssl=3"],
            "conflicts": ["openssl-1.1"],
            "replaces": ["libssl"]
        }"#;
        let v = parse_index_json(&format!("[{json}]")).unwrap();
        assert_eq!(v.len(), 1);
        let e = &v[0];
        assert_eq!(e.name, "openssl");
        assert_eq!(e.epoch, 1);
        assert_eq!(e.depends, vec!["zlib>=1.3".to_string(), "pcre2".to_string()]);
        assert_eq!(e.provides, vec!["libssl=3".to_string()]);
        assert_eq!(e.conflicts, vec!["openssl-1.1".to_string()]);
        assert_eq!(e.replaces, vec!["libssl".to_string()]);
    }

    #[test]
    fn parse_defaults() {
        // Minimal object: only the required fields.
        let json = r#"[{"name":"zlib","version":"1.3.1","release":1,
                        "sha256":"e5f6","url":"https://h/zlib-1.3.1-1.gpkg"}]"#;
        let v = parse_index_json(json).unwrap();
        assert_eq!(v[0].epoch, 0);
        assert_eq!(v[0].arch, "x86_64");
        assert_eq!(v[0].size, 0);
        assert!(v[0].depends.is_empty());
    }

    #[test]
    fn parse_empty_and_garbage() {
        assert!(parse_index_json("[]").unwrap().is_empty());
        assert!(parse_index_json("{}").is_err());
        assert!(parse_index_json("not json").is_err());
    }

    #[test]
    fn roundtrip_serializes_verbatim() {
        let obj = r#"{"name":"curl","version":"8.7.1","release":1,
                     "sha256":"a1","url":"https://h/curl.gpkg",
                     "depends":["openssl>=1.3"]}"#;
        let v = parse_index_json(&format!("[{obj}]")).unwrap();
        let out = serde_json::to_string(&v).unwrap();
        let v2 = parse_index_json(&out).unwrap();
        assert_eq!(v, v2, "serde round-trip must be lossless");
    }
}
