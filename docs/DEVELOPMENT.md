# Development

## Engine

Install stable Rust, a C compiler and the development packages used by CPAL.
For example, on Debian/Ubuntu:

```sh
sudo apt-get install build-essential pkg-config libasound2-dev libudev-dev
LD_LIBRARY_PATH="$PWD/data/runtime/linux-x86_64" cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These commands exercise the standalone Rust engine. They do not by themselves
compile the Horizon adapter or launch a game.

The test-only library path selects this repository's bundled Linux runtime.
Players do not need it: the launcher's post-build step installs libraries where
the game loads them by absolute path. CI also runs the codec/DSP self-check in
loopback mode without physical audio devices.

### Linux runtime portability

The bundled Opus 1.6.1 and RNNoise 0.2 use unmodified upstream release sources
and `CFLAGS="-O2 -march=x86-64 -mtune=generic"`. Build shared libraries without
host-specific ISA flags; keep Opus runtime CPU detection enabled and configure
RNNoise with `--enable-x86-rtcd`. Source checksums are in
`data/runtime/README.txt`; retain the upstream BSD notices.

Build/link against a GLIBC sysroot whose exported ABI is no newer than 2.29.
When rebuilding on a newer development machine, verify `readelf -V` output:
linking against its host libm can introduce newer versioned math symbols even
with baseline CPU flags. Exclude packed-relative-relocation requirements.
Update the runtime manifest's SHA-256 values after stripping the binaries.

Run the engine tests and installer self-check against the rebuilt libraries.
CI also runs `qemu-x86_64 -cpu qemu64 target/debug/plasmo-voice-runtime --check`
with the bundled library path, exercising a CPU without AVX. This check caught
SIGILL in the previous distribution-specific binaries. QEMU and the private
userspace used locally are test tools, not part of the distributed mod.

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

The output is `dist/plasmo-voice-0.5.2.hnmod`. To reproduce the release's core
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
