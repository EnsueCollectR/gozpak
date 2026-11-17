//! Output helpers mirroring the shell `log`/`war`/`die`.
//!
//! The shell version colors the `->` prefix on a TTY (auto-disabled when
//! piped); M1 always prints plain lines, which matches piped behavior.
//! All lines go to stderr, like the shell version.

/// `-> pkg detail` (detail omitted when empty).
pub fn log_line(pkg: &str, detail: &str) {
    if pkg.is_empty() && detail.is_empty() {
        eprintln!("->");
    } else if pkg.is_empty() {
        eprintln!("-> {detail}");
    } else if detail.is_empty() {
        eprintln!("-> {pkg}");
    } else {
        eprintln!("-> {pkg} {detail}");
    }
}

/// `WARNING pkg detail` (shell `war`: log with the WARNING prefix).
pub fn war(pkg: &str, detail: &str) {
    if pkg.is_empty() {
        eprintln!("WARNING {detail}");
    } else if detail.is_empty() {
        eprintln!("WARNING {pkg}");
    } else {
        eprintln!("WARNING {pkg} {detail}");
    }
}

/// `ERROR pkg detail` then exit(1) (shell `die`).
pub fn die(pkg: &str, detail: &str) -> ! {
    if pkg.is_empty() {
        eprintln!("ERROR {detail}");
    } else if detail.is_empty() {
        eprintln!("ERROR {pkg}");
    } else {
        eprintln!("ERROR {pkg} {detail}");
    }
    std::process::exit(1);
}
