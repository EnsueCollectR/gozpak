//! Extension fallback — the stub for M1.
//!
//! The shell version routes any unknown first word `X` to an extension:
//! it locates `gozpak-X` on `$PATH` and execs it (`pkg_find
//! "gozpak-$action*" "" -x "$PATH"`). M1 does not exec extensions yet —
//! that lands with M2 along with the build/install commands — so this
//! reports what *would* happen and exits non-zero.

use crate::log;

/// Locate `bin` on `$PATH` (first match, like the shell's `command -v`).
fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Handle an unknown first word as an extension call.
///
/// `names` are the would-be extension arguments (not forwarded in M1).
pub fn fallback(action: &str, names: &[String]) -> i32 {
    let bin = format!("gozpak-{action}");
    match which(&bin) {
        Some(path) => {
            log::log_line(
                action,
                &format!(
                    "{bin} found at {} but extension execution is not implemented in M1{}",
                    path.display(),
                    if names.is_empty() {
                        String::new()
                    } else {
                        format!(" (would pass: {})", names.join(" "))
                    }
                ),
            );
            1
        }
        None => log::die(
            action,
            &format!("{bin} not found in PATH (extension support is a M2 feature)"),
        ),
    }
}
