//! A tiny shell-style glob matcher for `gozpak search`.
//!
//! Supports `*`, `?`, and literal characters (the patterns actually used by
//! the shell version: bare package names and `gozpak-*` style extension
//! prefixes). `[...]` character classes are *not* supported in M1 and are
//! matched literally.

/// Does `name` match shell glob `pattern`?
pub fn glob_match(name: &str, pattern: &str) -> bool {
    let s: Vec<char> = name.chars().collect();
    let p: Vec<char> = pattern.chars().collect();
    let (n, m) = (s.len(), p.len());

    // dp[i][j]: does s[i..] match p[j..]?
    let mut dp = vec![vec![false; m + 1]; n + 1];
    dp[n][m] = true;
    // Empty string: only trailing stars can still match.
    for j in (0..m).rev() {
        dp[n][j] = p[j] == '*' && dp[n][j + 1];
    }
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = match p[j] {
                '*' => {
                    // Star matches empty, or it eats s[i] and retries.
                    dp[i][j + 1] || dp[i + 1][j]
                }
                '?' => dp[i + 1][j + 1],
                c => s[i] == c && dp[i + 1][j + 1],
            };
        }
    }
    dp[0][0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_names() {
        assert!(glob_match("mesa", "mesa"));
        assert!(!glob_match("mesa", "mesa3d"));
        assert!(!glob_match("mesa", "messa"));
    }

    #[test]
    fn wildcards() {
        assert!(glob_match("gozpak-foo", "gozpak-*"));
        assert!(glob_match("mesa", "mesa*"));
        assert!(glob_match("mesa", "*"));
        assert!(!glob_match("xfwm4", "gozpak-*"));
        assert!(glob_match("a1b", "a*b"));
    }

    #[test]
    fn question_mark() {
        assert!(glob_match("abc", "a?c"));
        assert!(!glob_match("abc", "a??c"));
        assert!(glob_match("ab", "a?"));
    }

    #[test]
    fn star_backtracking() {
        assert!(glob_match("prefix-mesa-suffix", "*mesa*"));
        assert!(glob_match("x", "x*"));
        assert!(glob_match("", "*"));
        assert!(!glob_match("nope", "mesa*"));
        assert!(glob_match("aaa", "a*a*a"));
        assert!(!glob_match("ab", "a*a*a"));
    }

    #[test]
    fn brackets_are_literal_in_m1() {
        // M1 limitation, documented: [set] is not interpreted.
        assert!(glob_match("[1]", "[1]"));
        assert!(!glob_match("1", "[1]"));
    }
}
