# Development

## Engine

Install stable Rust, a C compiler and the development packages used by CPAL.
For example, on Debian/Ubuntu:

```sh
sudo apt-get install build-essential pkg-config libasound2-dev libudev-dev
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These commands exercise the standalone Rust engine. They do not by themselves
compile the Horizon adapter or launch a game.

## Hooks/Host adapter

Clone Horizon next to this repository, then stage a disposable development
worktree. The explicit revision below reproduces the accepted-source check:

```sh
git clone --filter=blob:none https://github.com/horizon-rust/horizon ../horizon
python3 tools/prepare_native_api.py --horizon ../horizon \
  --revision ffd9ffb99554fc64433ca8a9258cf7a8ec1d5c2f
cargo test --manifest-path .local/native-api-accepted/Cargo.toml \
  -p horizon-plasmo-native --lib
cargo clippy --manifest-path .local/native-api-accepted/Cargo.toml \
  -p horizon-plasmo-native --all-targets -- -D warnings
```

`prepare_native_api.py` adds the adapter, engine, development wrapper and runtime
data to the disposable workspace. It does not patch the Horizon core. Use
`--refresh-adapter` to refresh mod sources in an existing staged workspace;
use a new `--out .local/<name>` to select a different core revision.

## Package

While the required API entries are unnumbered, use the explicitly named
source-check mode:

```sh
python3 tools/package_native_api.py --source .local/native-api-accepted --pinned
```

The output is `dist/plasmo-voice-0.5.0-pinned-check.hnmod`. After the official
catalog contains all required entries, use a clean source checkout with that
accepted stamp and omit `--pinned`:

```sh
python3 tools/package_native_api.py --source /path/to/registered/horizon
```

The tool reads the actual committed API level and creates the portable package;
it refuses to guess a number. Package output and SHA-256 are printed as JSON.
The registry's final file check is `cargo horizon-mod check <file.hnmod>`.

For playing, let the official launcher build in **Performance** mode. Keep live
tests isolated from real game data; prepare resources from your own Minecraft
copy. Do not commit or print account/session files. Device tests should use
explicitly configured virtual devices unless a human has chosen a physical test.

## Brand assets

The source icon and exact generation prompts are in `assets/`. Export required
sizes and the GitHub social preview with:

```sh
python3 tools/export_branding.py
```

This uses ImageMagick and `rsvg-convert` for deterministic size/layout exports.
The original illustration is kept unchanged. Assets are shared by README,
GitHub preview and the prepared Horizon catalog entry.
