//! Repository URL configuration (docs/repos.md).
//!
//! Precedence: the `GOZPAK_REPOS` environment variable (colon-separated)
//! takes priority over the config file `$GOZPAK_ROOT/etc/gozpak/repos.conf`
//! (one URL per line, `#` comments, blank lines ignored).

use std::path::Path;

use gozpak_core::paths;

/// Parse the `GOZPAK_REPOS` value: a colon-separated list of URLs. A `:`
/// that is part of `://` (scheme separator) does NOT split; only a `:`
/// not followed by `//` is a separator (so `https://a.org/r:https://b.org/m`
/// yields two URLs). Empty segments are dropped, whitespace trimmed.
///
/// (The shell v6.0.0 reader split on every `:`, which mangled `https://`
/// URLs — a latent bug this implementation fixes properly.)
pub fn parse_env_repos(value: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    let mut rest = value;
    while !rest.is_empty() {
        // Find the next separator: a ':' not followed by '//'.
        let sep = rest.char_indices().find_map(|(i, c)| {
            if c == ':' && !rest[i + 1..].starts_with("//") {
                Some(i)
            } else {
                None
            }
        });
        match sep {
            Some(i) => {
                urls.push(rest[..i].trim().to_string());
                rest = &rest[i + 1..];
            }
            None => {
                urls.push(rest.trim().to_string());
                rest = "";
            }
        }
    }
    urls.retain(|s| !s.is_empty());
    urls
}

/// Parse `repos.conf` text: one URL per line; lines starting with `#` and
/// blank lines are skipped.
pub fn parse_conf(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

/// Load and parse `$root/etc/gozpak/repos.conf` (missing file → empty).
pub fn load_conf(root: &Path) -> Vec<String> {
    match std::fs::read_to_string(paths::repos_conf(root)) {
        Ok(text) => parse_conf(&text),
        Err(_) => Vec::new(),
    }
}

/// Effective repository URLs: `GOZPAK_REPOS` env var wins, else the config
/// file (mirrors `repo_read_config`).
///
/// The env value is read from the process environment; tests should use
/// [`parse_env_repos`] / [`load_conf`] directly instead of mutating env.
pub fn configured_repos(root: &Path) -> Vec<String> {
    if let Ok(env) = std::env::var("GOZPAK_REPOS") {
        let urls = parse_env_repos(&env);
        if !urls.is_empty() {
            return urls;
        }
    }
    load_conf(root)
}

/// Filesystem-safe identifier for a repository URL: strip the
/// `scheme://`, replace `/`, `:` and `.` with `_`, and trim leading/trailing
/// underscores (e.g. `file:///x/y` → `x_y`, not `_x_y`).
///
/// ```text
/// https://repo.gozjaro.org/stable → repo_gozjaro_org_stable
/// ```
pub fn url_id(url: &str) -> String {
    let without_scheme = match url.find("://") {
        Some(i) => &url[i + 3..],
        None => url,
    };
    without_scheme
        .chars()
        .map(|c| match c {
            '/' | ':' | '.' => '_',
            c => c,
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_repos_parsing() {
        assert_eq!(
            parse_env_repos("https://a.org/stable:https://b.org/mirror"),
            vec![
                "https://a.org/stable",
                "https://b.org/mirror"
            ]
        );
        // empty segments dropped; whitespace trimmed
        assert_eq!(
            parse_env_repos("https://a.org : :https://b.org"),
            vec!["https://a.org", "https://b.org"]
        );
        assert!(parse_env_repos("").is_empty());
    }

    #[test]
    fn conf_parsing() {
        let text = "\
# Official repository
https://repo.gozjaro.org/stable

# Community mirror
https://mirror.example.com/gozjaro
";
        assert_eq!(
            parse_conf(text),
            vec![
                "https://repo.gozjaro.org/stable",
                "https://mirror.example.com/gozjaro"
            ]
        );
        assert!(parse_conf("# only a comment\n\n").is_empty());
    }

    #[test]
    fn load_conf_missing_file_is_empty() {
        assert!(load_conf(Path::new("/definitely/does/not/exist")).is_empty());
    }

    #[test]
    fn url_id_matches_shell() {
        // shell: `${url#*://}` (shortest prefix through `://`) + fnr '/' ':' '.' -> '_'
        assert_eq!(
            url_id("https://repo.gozjaro.org/stable"),
            "repo_gozjaro_org_stable"
        );
        assert_eq!(url_id("https://host:8080/a.b"), "host_8080_a_b");
        // file:///x/y → first "://" at index 4, remainder "x/y"
        assert_eq!(url_id("file:///x/y"), "x_y");
    }
}
