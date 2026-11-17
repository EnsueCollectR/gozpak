# gozpak (Rust) — M1

Rust rewrite of the Gozjaro `gozpak` package manager, phase **M1**
(read-only CLI parity). Tracked as
[gozjaro-bootstrap#93](https://github.com/Gozjaro/gozjaro-bootstrap/issues/93)
(milestone 26.11), in the `Gozjaro/gozpak` org alongside the shell
`gozpak` v6.0.0 this mirrors.

M1 is **read-only**: no network, no build/install/remove. Those arrive in
M2/M3 (see [roadmap](#m2m3-roadmap)).

## Workspace layout

```
rs/
├── Cargo.toml              # workspace (edition 2021, MSRV 1.85, v6.0.0)
└── crates/
    ├── gozpak-core/        # VersionRel, Pkg, Dependency, InstalledDb, paths, SHA-256
    ├── gozpak-recipe/      # recipe parsing: recipe.toml (new primary) + legacy 3-file
    ├── gozpak-repo/        # repo.index parse/serialize + GOZPAK_REPOS / repos.conf
    └── gozpak-cli/         # the `gozpak` binary (lib + thin main)
```

Dependencies: **none** in `gozpak-core`/`gozpak-repo`/`gozpak-cli`
(SHA-256 is a small verified in-house FIPS 180-4 implementation, matching
the shell version's "no standard sha256 utility" problem). The only external
crates in the workspace are `toml` + `serde`/derive, confined to
`gozpak-recipe`, which is where the new `recipe.toml` format is parsed.

## M1 commands

| command (alias) | status |
|---|---|
| `version` (v) | implemented — prints `6.0.0` (parity with the shell) |
| `list` (l) | implemented — installed packages from `$GOZPAK_ROOT/var/db/gozpak/installed` |
| `search` (s) | implemented — recipe dirs under `GOZPAK_PATH` (+ installed DB root), shell-glob queries |
| `checksum` (c) | implemented — hashes locally-resolvable sources (local files + cached downloads); remote-only recipes are no-ops until M2 downloads |
| `install`/`build`/`get`/`remove`/`download`/`fetch`/`sync`/`list-remote`/`update`/`upgrade`/`alternatives` | stubs — log `not implemented in M1`, exit 1 |
| unknown word | extension fallback stub — locates `gozpak-<word>` on `$PATH`, reports it, refuses to exec (M2) |

Global `-y`/`--yes` flag is parsed (recorded as `Ctx::yes`). Output style
mirrors the shell `log`/`war`/`die` lines (`-> pkg detail` / `WARNING ...` /
`ERROR ...`, all on stderr; M1 drops TTY color).

### Environment

* `GOZPAK_ROOT` — system root (default `/`); the installed DB lives at
  `$GOZPAK_ROOT/var/db/gozpak/installed`, repo config at
  `$GOZPAK_ROOT/etc/gozpak/repos.conf`.
* `GOZPAK_PATH` — colon-separated recipe search path for `search`/`checksum`.
* `GOZPAK_REPOS` — colon-separated repo URLs (takes priority over
  `repos.conf`); parsed by `gozpak-repo::urls`, consumed by `sync`/`get`
  in M2.

### No-argument parity detail

Like the shell version, `search`/`checksum` with no package arguments use
the current directory's basename as the package name and prepend its parent
directory to `GOZPAK_PATH` — so `gozpak c` works from inside a recipe dir.

## Recipe formats

`Recipe::load(dir)` prefers the **new primary format**, falling back to the
legacy layout, and records which one it used (`Recipe::kind`):

* **`recipe.toml`** (new primary, parsed via the `toml` crate):
  `[package]` (name/version/release/arch/desc/license/depends/build_depends),
  `[source]` (url with `$VERSION`/`$RELEASE`/`$MAJOR`/`$MINOR`/`$PATCH`/
  `$PACKAGE` substitution, optional `sha256`, `git+` prefix, `patches[]`),
  `[build]` (`system = raw|autotools|meson|cmake|make`, `steps[]` shell
  lines with `$DESTDIR`/`$VERSION`, optional `meson_args[]`/`cmake_args[]`),
  `[install]` (`post[]` hook names or shell strings, `files = "auto"`),
  `[meta]` (free-form key=value). See
  `crates/gozpak-recipe/tests/fixtures/toml-mesa/recipe.toml`.
* **Legacy 3-file** (`version`, `build`, `depends`, `sources`, `meta`,
  `checksums`) — kept via `Recipe::load_legacy(dir)`; it will power the M3
  migration tool. Shell-style bare-token substitution (`acl-VERSION.tar.xz`)
  is preserved for legacy source lines; TOML URLs use `$VERSION` style.

Fixtures for both formats live in `crates/gozpak-recipe/tests/fixtures/`
(`toml-mesa` mirrors a mesa-like recipe, `legacy-acl` mirrors
gozjaro-repo's `acl`).

## Building & testing

```sh
cd rs
cargo build          # → target/debug/gozpak
cargo test           # unit + integration tests across all crates
./target/debug/gozpak version   # → 6.0.0
```

## M2/M3 roadmap

* **M2 — write side:** `build`/`install` (execute `[build].steps` / legacy
  shell `build` with the documented env), `remove`, `get`/`fetch`/`sync`
  (network: download `repo.index` + tarballs, verify against
  `IndexEntry.sha256`), extension execution, `update`/`upgrade` (via repo
  diffing), TTY colors, `GOZPAK_HOOK` support.
* **M3 — migration & publishing:** `gozpak-recipe` migration tool that
  converts legacy 3-file recipes to `recipe.toml` (using
  `Recipe::load_legacy`), and `repo-build`/`repo-index`/`repo-push`
  (see `contrib/gozpak-repo-index`).
