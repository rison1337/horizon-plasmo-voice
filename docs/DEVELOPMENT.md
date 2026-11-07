# Development

## Engine

Install stable Rust, C/C++ compilers, CMake and the development packages used
by CPAL. For example, on Debian/Ubuntu:

```sh
sudo apt-get install build-essential cmake pkg-config libasound2-dev libudev-dev
cargo test --workspace --locked
cargo test --workspace --test hrtf --test denoise -- --ignored
cargo run --locked -p horizon-plasmo-engine --bin plasmo-voice-runtime -- --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These commands exercise the standalone engine; they do not launch a game.
No system Opus, RNNoise or OpenAL install or library-path override is required.

### Reviewable native dependencies

`runtime/build.rs` verifies the SHA-256 of the three immutable upstream source
archives in `runtime/data/sources/`, extracts them into Cargo's `OUT_DIR`, and
runs CMake to compile shared libraries for the target. Its build does not fetch
anything from the network. `runtime/data/sources.json` lists original URLs,
versions, licences and archive hashes. The unmodified source archives are the
reviewable payload; no `.so`, `.dll`, object or executable is bundled.

Opus 1.6.1 and RNNoise 0.2 use generic scalar C; baseline x86_64 flags avoid
host-specific CPU instructions. OpenAL Soft 1.25.2 retains upstream runtime
CPU dispatch and its embedded HRTF dataset, with only loopback/null backends.
CPAL continues to own the user's recording and playback devices. OpenAL stays
a separately loaded LGPL library; no implementation is copied into Rust.

The post-build installer embeds only the libraries produced by that Cargo
build. It installs those outputs, licence notices, and complete corresponding
source archives beside the executable, then tests Opus/RNNoise/HRTF without
physical devices. Its receipt records hashes of installed outputs. The loader
uses absolute installed paths, or that build's output for development; it never
falls back to an arbitrary system codec or DLL on PATH.

Source builds use the player's local C/C++ ABI, rather than distributing a
precompiled library tied to the developer's GLIBC. Linux needs the headers
above. Windows x64 GNU needs MinGW-w64 C/C++ and CMake and Ninja on PATH (the launcher
provides its Rust/MinGW toolchain; install CMake and Ninja if absent). Windows MSVC and
macOS are not validated targets. CI checks baseline CPU codec/DSP loopback.

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

The official API minimum is **411**, registered by Horizon PR #9812. Build a
release from its clean public source, or a compatible later source whose actual
API level you have tested:

```sh
python3 tools/package_native_api.py --source /path/to/registered/horizon
```

The output is `dist/plasmo-voice-0.5.3.hnmod`. To reproduce the release's core
revision, stage it explicitly:

```sh
python3 tools/prepare_native_api.py --horizon ../horizon \
  --revision d52ff7a6f783a164043549f5f04645d5324d0d8a --out .local/native-api-release
python3 tools/package_native_api.py --source .local/native-api-release
```

The tool reads the actual committed API level and creates the portable package;
it refuses to guess a number. Package output and SHA-256 are printed as JSON.
The registry's final file check is `cargo horizon-mod check <file.hnmod>`.
The explicit `--pinned` option remains available for development-only checks on
older source whose required APIs are still pending; it is not a release mode.

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
