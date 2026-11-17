//! Recipe parsing tests: TOML v1.1 (new primary) and legacy (kiss-style)
//! fixtures.

use std::path::{Path, PathBuf};

use gozpak_core::{DepReq, Op};
use gozpak_recipe::{glob, subst, HookSpec, Recipe, RecipeError, RecipeKind, Source};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("gozpak-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

// ---------------------------------------------------------------------------
// TOML v1.1 fixtures (openssl-like, mirroring docs/recipe-format.md)
// ---------------------------------------------------------------------------

#[test]
fn toml_openssl_loads() {
    let dir = fixtures().join("toml-openssl");
    let r = Recipe::load(&dir).expect("toml-openssl should load");

    assert_eq!(r.kind, RecipeKind::Toml);
    assert_eq!(r.name, "openssl");
    assert_eq!(r.version.display(), "3.5.1-1");
    assert_eq!(r.epoch, 1);
    assert_eq!(r.arch.as_deref(), Some("x86_64"));
    assert_eq!(r.desc.as_deref(), Some("OpenSSL toolkit"));
    assert_eq!(r.license.as_deref(), Some("Apache-2.0"));
    assert_eq!(r.homepage.as_deref(), Some("https://www.openssl.org/"));

    // v1.1 constraint syntax on depends.
    assert_eq!(r.depends.len(), 2);
    assert_eq!(r.depends[0].name, "zlib");
    assert_eq!(r.depends[0].op, Some(Op::Ge));
    assert_eq!(r.depends[0].version.as_deref(), Some("1.3"));
    assert_eq!(r.depends[1].name, "pcre2");
    assert!(r.depends[1].op.is_none());
    assert!(r.depends.iter().all(|d| !d.build_only));

    // build_depends: constraint + build-only flag.
    assert_eq!(r.build_depends.len(), 2);
    assert_eq!(r.build_depends[0].name, "cmake");
    assert_eq!(r.build_depends[0].op, Some(Op::Ge));
    assert!(r.build_depends.iter().all(|d| d.build_only));

    // v1.1 solver fields.
    assert_eq!(r.provides.len(), 1);
    assert_eq!(r.provides[0].name, "libssl");
    assert_eq!(r.provides[0].op, Some(Op::Eq));
    assert!(r.conflicts.is_empty());
    assert!(r.replaces.is_empty());

    // v1.1 [install]: mixed post array (string + inline table), backup.
    assert_eq!(
        r.install_post,
        vec![
            HookSpec {
                hook: "ldconfig".into(),
                match_filter: None,
            },
            HookSpec {
                hook: "update-mime-database".into(),
                match_filter: Some("/usr/share/mime/**".into()),
            }
        ]
    );
    assert_eq!(r.install_files, "auto");
    assert_eq!(r.install_backup, vec!["etc/ssl/openssl.cnf"]);

    assert_eq!(
        r.meta.get("maintainer").map(String::as_str),
        Some("gozjaro@example.org")
    );
    // No warnings: arch is explicit x86_64, sha256 present.
    assert!(r.warnings.is_empty(), "warnings: {:?}", r.warnings);
}

#[test]
fn toml_openssl_sources_normalized() {
    let dir = fixtures().join("toml-openssl");
    let r = Recipe::load(&dir).unwrap();

    // One Files source (the urls[] entry) + the local patch.
    assert_eq!(r.sources.len(), 2);
    match &r.sources[0] {
        Source::Files { files } => {
            assert_eq!(files.len(), 1);
            assert!(files[0].url.contains("openssl-$VERSION.tar.xz"));
            assert!(files[0].sha256.is_some());
            assert!(files[0].dest.is_none());
        }
        other => panic!("expected Files source, got {other:?}"),
    }
    match &r.sources[1] {
        Source::Local { path } => {
            assert_eq!(path, Path::new("patches/fix-ssl3.patch"));
        }
        other => panic!("expected local patch, got {other:?}"),
    }

    // Dollar-style substitution on the TOML url, incl. $EPOCH (v1.1).
    let s = subst::Subst::from_recipe(&r.name, &r.version, r.epoch);
    let substituted = s.apply(&r.sources[0].display());
    assert!(substituted.contains("openssl-3.5.1.tar.xz"), "{substituted}");
    assert!(!substituted.contains("$VERSION"), "{substituted}");
}

#[test]
fn toml_git_source_loads() {
    let dir = fixtures().join("toml-git");
    let r = Recipe::load(&dir).expect("toml-git should load");

    assert_eq!(r.arch.as_deref(), Some("x86_64"), "arch defaults to x86_64 (v1.1)");
    assert_eq!(r.sources.len(), 1);
    match &r.sources[0] {
        Source::Git { repo, pin } => {
            assert_eq!(repo.as_str(), "https://github.com/Gozjaro/gozjaro-installer-rs");
            assert_eq!(pin.as_deref(), Some("d41d8cd9"));
        }
        other => panic!("expected git source, got {other:?}"),
    }
    assert!(r.warnings.is_empty(), "git sources need no sha256: {:?}", r.warnings);
}

#[test]
fn toml_git_ref_must_be_pinned() {
    let dir = tmp("git-bad-ref");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n[source]\n\
         git = \"git+https://github.com/org/repo#main\"\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("branch"), "message: {message}");
        }
        other => panic!("expected pinned-ref error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);

    // No ref at all is also an error.
    let dir = tmp("git-no-ref");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n[source]\n\
         git = \"https://github.com/org/repo\"\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("pinned ref"), "message: {message}");
        }
        other => panic!("expected pinned-ref error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn toml_validation_errors() {
    // missing [package].version
    let dir = tmp("toml-bad-version");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\narch = \"x86_64\"\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("version"), "message: {message}");
        }
        other => panic!("expected version error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);

    // invalid arch
    let dir = tmp("toml-bad-arch");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\narch = \"riscv64\"\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("arch"), "message: {message}");
        }
        other => panic!("expected arch error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);

    // raw build system without steps
    let dir = tmp("toml-bad-steps");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n[build]\nsystem = \"raw\"\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("steps"), "message: {message}");
        }
        other => panic!("expected steps error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);

    // urls[] and git together
    let dir = tmp("toml-both-sources");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n\
         [source]\nurls = [{ url = \"https://host/x.tar.xz\", sha256 = \"ab\" }]\n\
         git = \"git+https://github.com/org/repo#1.0\"\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("not both"), "message: {message}");
        }
        other => panic!("expected both-sources error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);

    // non-registry hook name
    let dir = tmp("toml-bad-hook");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n\
         [install]\npost = [\"evil-hook\"]\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("registry"), "message: {message}");
        }
        other => panic!("expected hook error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);

    // shell-looking post entry
    let dir = tmp("toml-shell-hook");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n\
         [install]\npost = [\"ldconfig; rm -rf /\"]\n",
    )
    .unwrap();
    match Recipe::load(&dir) {
        Err(RecipeError::Toml { message, .. }) => {
            assert!(message.contains("shell"), "message: {message}");
        }
        other => panic!("expected shell-hook error, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn toml_missing_sha256_warns() {
    let dir = tmp("toml-no-sha");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"x\"\nversion = \"1.0\"\nrelease = 1\n\n\
         [source]\nurls = [{ url = \"https://host/x.tar.xz\" }]\n",
    )
    .unwrap();
    let r = Recipe::load(&dir).expect("missing sha256 is a warning in M1, not an error");
    assert!(
        r.warnings.iter().any(|w| w.contains("sha256")),
        "warnings: {:?}",
        r.warnings
    );
    match &r.sources[0] {
        Source::Files { files } => assert!(files[0].sha256.is_none()),
        other => panic!("expected Files source, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn toml_any_arch_warns() {
    let dir = tmp("toml-any-arch");
    std::fs::write(
        dir.join("recipe.toml"),
        "[package]\nname = \"wallpapers\"\nversion = \"1.0\"\nrelease = 1\n\
         arch = \"any\"\ndesc = \"data package\"\n",
    )
    .unwrap();
    let r = Recipe::load(&dir).unwrap();
    assert_eq!(r.arch.as_deref(), Some("any"));
    assert!(
        r.warnings.iter().any(|w| w.contains("\"any\"")),
        "warnings: {:?}",
        r.warnings
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_prefers_toml_over_legacy() {
    // A directory with BOTH recipe.toml and legacy files must load as TOML.
    let mixed = tmp("mixed");
    std::fs::write(
        mixed.join("recipe.toml"),
        "[package]\nname = \"mixed\"\nversion = \"3.0\"\nrelease = 1\n",
    )
    .unwrap();
    std::fs::write(mixed.join("version"), "9.9 9\n").unwrap();
    let r = Recipe::load(&mixed).unwrap();
    assert_eq!(r.kind, RecipeKind::Toml);
    assert_eq!(r.version.display(), "3.0-1");
    let _ = std::fs::remove_dir_all(&mixed);
}

// ---------------------------------------------------------------------------
// Legacy fixtures (acl / hello style, mirroring gozjaro-repo)
// ---------------------------------------------------------------------------

#[test]
fn legacy_acl_loads() {
    let dir = fixtures().join("acl");
    let r = Recipe::load_legacy(&dir).expect("legacy acl should load");

    assert_eq!(r.kind, RecipeKind::Legacy);
    assert_eq!(r.name, "acl");
    assert_eq!(r.version.display(), "2.3.2-1");
    assert_eq!(r.epoch, 0);
    assert!(r.arch.is_none(), "legacy recipes carry no arch");
    assert_eq!(
        r.depends.iter().map(|d| d.name.clone()).collect::<Vec<_>>(),
        vec!["attr".to_string()]
    );
    assert_eq!(
        r.build_depends
            .iter()
            .map(|d| d.name.clone())
            .collect::<Vec<_>>(),
        vec!["ninja".to_string()]
    );
    assert!(r.build_depends.iter().all(|d| d.build_only));
    assert_eq!(r.desc.as_deref(), Some("Access control list utilities"));
    assert_eq!(r.license.as_deref(), Some("LGPL-2.1-or-later"));
    assert_eq!(r.meta.get("type").map(String::as_str), Some("library"));
    assert_eq!(r.checksums, vec!["SKIP".to_string()]);
    assert_eq!(r.build_steps.len(), 1);
    assert!(r.build_steps[0].starts_with("#!/bin/sh"));

    // Legacy remote line normalizes into a one-file Sources entry.
    assert_eq!(r.sources.len(), 1);
    match &r.sources[0] {
        Source::Files { files } => {
            assert_eq!(files.len(), 1);
            assert!(files[0].url.contains("acl-VERSION.tar.xz"));
            assert!(files[0].sha256.is_none());
        }
        other => panic!("expected files source, got {other:?}"),
    }

    // The shell-style source line substitutes bare VERSION markers.
    let s = subst::Subst::from_recipe(&r.name, &r.version, r.epoch);
    let line = r.sources[0].display();
    assert_eq!(
        s.apply_shell(&line),
        "https://download.savannah.gnu.org/releases/acl/acl-2.3.2.tar.xz"
    );
}

#[test]
fn legacy_missing_version_is_error() {
    let dir = fixtures().join("local-hello");
    // local-hello HAS a version file; build a broken copy instead.
    let broken = tmp("no-ver");
    std::fs::write(broken.join("build"), "#!/bin/sh\nexit 0\n").unwrap();
    match Recipe::load_legacy(&broken) {
        Err(RecipeError::MissingVersion(p)) => {
            assert!(p.to_string_lossy().ends_with("version"));
        }
        other => panic!("expected MissingVersion, got {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&broken);

    // And the fixture with a version file loads fine:
    let r = Recipe::load_legacy(&dir).unwrap();
    assert_eq!(r.name, "local-hello");
    assert_eq!(r.version.display(), "1.0-1");
    assert_eq!(r.sources.len(), 1);
    match &r.sources[0] {
        Source::Local { path } => {
            assert_eq!(path, Path::new("src/hello.txt"));
        }
        other => panic!("expected local source, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Search / find
// ---------------------------------------------------------------------------

#[test]
fn search_and_find() {
    let bases = vec![fixtures().to_path_buf()];

    // find(): exact directory-name match.
    let found = Recipe::find("toml-openssl", &bases).expect("find toml-openssl");
    assert!(found.is_dir());

    // search(): glob against directory names under the base.
    let hits = Recipe::search("toml-*", &bases);
    let names: Vec<_> = hits
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["toml-git".to_string(), "toml-openssl".to_string()]);

    let all = Recipe::search("*", &bases);
    assert!(all.len() >= 4, "found {all:?}");

    let none = Recipe::search("nope-*", &bases);
    assert!(none.is_empty());

    // glob module sanity.
    assert!(glob::glob_match("gozpak-sync", "gozpak-*"));
    assert!(!glob::glob_match("sync", "gozpak-*"));
}

// ---------------------------------------------------------------------------
// Local resolution
// ---------------------------------------------------------------------------

#[test]
fn resolve_local_sources() {
    let r = Recipe::load_legacy(&fixtures().join("local-hello")).unwrap();
    let cache = tmp("no-cache");

    let resolved = subst::resolve_local(&r.sources[0], &r.dir, &cache, &r.name);
    let subst::Resolved::File(p) = resolved else {
        panic!("local source should resolve to a file");
    };
    assert!(p.is_file());
    assert!(p.to_string_lossy().ends_with("hello.txt"));

    // An uncached remote source is Unavailable in M1 (no network).
    let acl = Recipe::load_legacy(&fixtures().join("acl")).unwrap();
    let r2 = subst::resolve_local(&acl.sources[0], &acl.dir, &cache, &acl.name);
    assert_eq!(r2, subst::Resolved::Unavailable);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn dep_req_display_roundtrip() {
    let d = DepReq::parse_constraint("zlib>=1.3", false).unwrap();
    assert_eq!(d.display(), "zlib>=1.3");
    assert_eq!(d.raw, "zlib>=1.3");
}
