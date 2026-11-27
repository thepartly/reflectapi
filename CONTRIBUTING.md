# Contributing

How to build, test, document and release reflectapi.

## Commits

Commit as yourself: your name and email as the author and committer. If an AI
agent writes a change, the commit is still yours, without its session trailer,
session link, co-author trailer or "Generated with" footer. The `Commit
metadata` check fails a pull request whose commits break this, and a push to
`main` that does, which covers a merge or squash commit whose message is
written when merging. See `.github/scripts/check-commit-metadata.py`. It runs
as it is on `main`, so a change to it takes effect once merged.

## Development environment

`flake.nix` holds the toolchain CI uses: Rust from `rust-toolchain.toml` with
the wasm32 target, the biome and ruff releases codegen's output depends on,
node, pnpm, uv and mdBook. You need:

- Nix with flakes (`experimental-features = nix-command flakes`), on any
  Linux or macOS.
- direnv with nix-direnv, to load the shell on `cd` (`direnv allow` once).
  On NixOS: `programs.direnv = { enable = true; nix-direnv.enable = true; };`
  Without direnv, run `nix develop`.
- On NixOS, `programs.nix-ld.enable = true`: the Python tests run on
  interpreters uv downloads, and editors' bundled language servers are
  prebuilt binaries too.
- In VS Code, the direnv extension (recommended in `.vscode/extensions.json`),
  so rust-analyzer finds the toolchain.

Without Nix, install biome 1.8.3 and ruff 0.15.8 (the versions in `flake.nix`)
on your PATH. Other versions format generated code differently, and the
snapshot tests fail.

## Building and running

To run the demo server:

```
cargo run --bin reflectapi-demo
```

To generate client in Typescript for demo server:

```
cargo run --bin reflectapi -- codegen --language typescript --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/typescript
```

To run the Typescript generated client. Note: requires the demo server running

```
cd reflectapi-demo/clients/typescript/
npm install
npm run start
```

To generate client in Rust for demo server:

```
cargo run --bin reflectapi -- codegen --language rust --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/rust/generated/src/
```

To run the Rust generated client. Note: requires the demo server running

```
cd reflectapi-demo/clients/rust/
cargo run --all-features
```


To generate client in Python for demo server:

```
cargo run --bin reflectapi -- codegen --language python --schema reflectapi-demo/reflectapi.json --output reflectapi-demo/clients/python/api_client --python-package-name api_client --python-sync
```

## Updating Snapshots

This project uses `insta` for snapshot testing to ensure code generation output is correct and stable. When tests fail due to snapshot mismatches:

1. Review the changes first to ensure they are expected
2. Update snapshots using one of these commands:

```bash
# Interactive review (recommended)
cargo insta review

# Auto-accept all changes (use with caution)
cargo insta accept
```

3. Re-run tests to verify they pass:

```bash
cargo test
```

## Building Documentation

```bash
# Install mdBook (one-time setup)
cargo install mdbook --locked

# Test the book's Rust examples: each page's `rust` blocks run as doctests
cargo test --manifest-path docs/doctests/Cargo.toml --doc

# Serve the whole site locally, as it is published: the book, the landing
# page and the extras. It rebuilds on every change and reloads open pages.
python3 docs/serve.py  # http://127.0.0.1:3000/

# Or build it once, as CI does: the book, then the landing page, sitemap.xml,
# llms.txt and llms-full.txt. build-extras.py also checks the form of the HTTP
# API guidelines' rules.
(cd docs && mdbook build)
python3 docs/build-extras.py
```

`mdbook serve` in `docs/` serves the book alone, without the landing page.

The logo files are generated; see [docs/graphics](docs/graphics/README.md).

## Releasing

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which:

1. Requires successful `CI` on a push to `main` for the exact tagged commit, verifies every published package version and internal dependency version matches the tag, and extracts a nonempty matching section from `CHANGELOG.md` (`packaging.version` parses both Cargo SemVer and PEP 440 forms).
2. Packages and verifies all five Rust crates with the locked dependencies and all features, and builds the pure-Python `reflectapi-runtime` sdist and wheel before either registry publishes.
3. Publishes the five Rust crates to crates.io in dependency order — `reflectapi-schema` → `reflectapi-derive` → `reflectapi` → `reflectapi-conventions` → `reflectapi-cli` — using crates.io Trusted Publishing (OIDC, no `CARGO_REGISTRY_TOKEN` secret), and publishes the Python runtime via PyPI Trusted Publishing. Cargo verifies each crate before publishing. A crate version already on crates.io is skipped (`.github/scripts/publish-crate.sh`), so a release that failed partway is finished by re-running its failed jobs.
4. Cuts a GitHub Release with the curated changelog section; pre-release versions are flagged automatically.

Both publishers run inside the `release` GitHub Environment, which is configured as a trusted publisher on each crate (crates.io) and on the `reflectapi-runtime` PyPI project. crates.io sets up trusted publishing only for a crate that exists, so the release stops at a new crate with an error saying so. Publish its first version by hand with an API token, from the tagged commit (`cargo publish -p <crate> --locked`), set up trusted publishing on it, and re-run the failed jobs.

To prepare a release, bump every version in lockstep, update the internal Cargo dependency versions and lockfiles, and move the release notes under a matching `## <version>` heading in `CHANGELOG.md`. The Cargo files use SemVer (`0.18.0-alpha.1`), `reflectapi-python-runtime/pyproject.toml` and `src/reflectapi_runtime/__init__.py` use PEP 440 (`0.18.0a1`):

```fish
# 1. Bump Cargo SemVer in all five crates.
cargo release \
    --exclude reflectapi-demo \
    --exclude reflectapi-demo-client \
    --exclude reflectapi-demo-client-generated \
    minor --execute --no-publish --no-tag

# 2. Bump the Python runtime (PEP 440 form) by hand:
#    reflectapi-python-runtime/pyproject.toml         version = "..."
#    reflectapi-python-runtime/src/reflectapi_runtime/__init__.py  __version__ = "..."

# 3. Verify every publishable crate, using Cargo's staged workspace registry:
cargo package --workspace --all-features --locked --allow-dirty \
    --exclude reflectapi-demo \
    --exclude reflectapi-demo-client \
    --exclude reflectapi-demo-client-generated

# 4. Commit and land the release preparation on main:
git commit -am "chore: <version>"

# 5. Wait for successful CI on the exact main commit to release.
#    Then, once publishing is approved, tag that commit and push the tag:
git tag v<version>          # the `v` prefix is required
git push origin v<version>
```

For pre-releases use `--prerelease alpha` (or `beta` / `rc`) on `cargo release`; the Python equivalent is `0.18.0a1`, `0.18.0b1`, `0.18.0rc1`.
