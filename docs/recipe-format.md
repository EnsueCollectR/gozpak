# gozpak recipe format (26.11, single-file TOML)

Supersedes the kiss-style 3-file recipe (`build` + `version` + `depends` + `sources`).
The old format is still readable (`Recipe::load_legacy`) until the migration tool
ships in M3; new recipes MUST use this format.

A recipe is a directory containing one `recipe.toml` (TOML 1.0) plus optional
`patches/` and other local source files.

## Schema

```toml
[package]
name     = "openssl"            # required, [a-z0-9-]+
version  = "3.5.1"             # required
release  = 1                   # required, bump when the recipe itself changes
arch     = "any"               # "x86_64" | "any" (default "any")
desc     = "..."               # one-line description
license  = "Apache-2.0"
url      = "https://www.openssl.org/"
depends        = ["zlib"]      # runtime deps (plain names)
build_depends  = ["cmake", "make"]

[source]
url    = "https://www.openssl.org/source/openssl-$VERSION.tar.xz"
sha256 = "..."                # recommended
# git sources:
# url = "git+https://github.com/user/repo#v1.0"
patches = ["patches/fix-build.patch"]   # applied in order, relative to recipe dir

[build]
system = "autotools"          # "raw" | "autotools" | "meson" | "cmake" | "make"
# system = "raw": the shell script lines run in order ($DESTDIR = $1, $VERSION exported):
steps = [
    "./configure --prefix=/usr --shared",
    "make -j$JOBS",
    "make DESTDIR=$DESTDIR install",
]
# system = "meson"/"cmake"/"autotools"/"make": the builder drives the standard flow;
# extra flags:
meson_args = ["-Dssl=true", "-Dtests=false"]
cmake_args = ["-DCMAKE_INSTALL_PREFIX=/usr"]
configure_args = ["--prefix=/usr"]
make_target = "install"

[install]
# post-install hooks: registry names or shell strings.
# registry: ldconfig, update-mime-database, gdk-pixbuf-query-loaders,
#           desktop-database, fontconfig, shared-mime-info
post = ["ldconfig", "update-mime-database /usr/share/mime"]
files = "auto"                # track installed file list for the pkg db

[meta]                        # all optional, free-form
maintainer = "..."
repo = "main"                 # repo channel
notes = "..."
```

## Variable substitution

Available in `[source].url`, `sha256`, and `[build].*`:

| Variable | Value |
|----------|-------|
| `$VERSION`  | full version string |
| `$RELEASE`  | release number |
| `$MAJOR`/`$MINOR`/`$PATCH` | version components |
| `$PACKAGE`  | package name |
| `$DESTDIR`  | staging dir (build/install phase only) |
| `$JOBS`     | parallelism (build phase only) |

## Binary packages (.gpkg)

`gozpak build`/`repo-build` packages a recipe into:

```
gozjaro-<name>-<version>-<release>.gpkg     # tar.gz
├── manifest.toml        # serialized recipe [package] + [install].post +
│                       # arch, sha256(rootfs), file count/manifest, repo, built-by
└── rootfs/              # staged install tree (files under $DESTDIR)
```

The repo index (`repo.index`) stays the current text format for M2; each line carries
`name version release arch sha256 url` (see contrib/gozpak-repo-index). The index and
manifest share the TOML/line families so one parser covers both.

## Legacy (kiss 3-file) — read-only until M3

`build` (sh, `$1`=DESTDIR, `$2`=version), `version` (`<ver> <rel>`),
`depends` (one per line, trailing ` make` = build-only), `sources` (URLs + `git+`),
`meta` (key=value). `Recipe::load` prefers `recipe.toml`; a legacy dir is loaded
with `RecipeKind::Legacy` and the CLI prints a one-line legacy warning.

Migration: `gozpak migrate <dir>` (M3) rewrites legacy → recipe.toml;
`build` file content maps to `[build] system="raw"` `steps`, the ` make`
suffix in `depends` splits into `build_depends`.
