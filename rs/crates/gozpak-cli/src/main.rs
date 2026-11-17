//! The `gozpak` binary (M1).
//!
//! All behavior lives in the `gozpak_cli` library so it can be unit-tested;
//! this binary is a thin wrapper that reads the environment, dispatches, and
//! exits with the returned code.

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    std::process::exit(gozpak_cli::run(&argv[1..]));
}
