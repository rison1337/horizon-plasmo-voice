# Development — SDK 0.6.0

`mod/` is the complete distributable flat project. It uses only the public
`horizon-mod-api` dependency (`mod_api`) fetched by the official checker into
`mod/.horizon/sdk/`. The manifest pins build
`f00125f61f794aaa3d4e950d301f353b35b6ee3d` and Mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`; the published SDK selects Rust 1.99.1.
No core patches, internals facade, injections, native-hooks.json, mod build.rs,
custom game distribution or mod-owned SDK rebuild is used.

## Engine checks

Install C/C++, CMake, pkg-config and Linux ALSA development headers. Windows GNU
also needs Ninja. The development workspace uses exactly `mod/src/voice/`:

```sh
cargo test --workspace --locked
cargo test --workspace --locked --test hrtf --test denoise -- --ignored
ALSOFT_DRIVERS=null cargo test --workspace --locked --test device_output -- --ignored
cargo run --locked -p horizon-plasmo-engine --bin plasmo-voice-runtime -- --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Tests use generated signals, loopback DSP and the explicitly selected null output,
not physical devices. Use `PLASMO_AUDIO_CACHE=/absolute/scratch/path` to isolate
prepared audio. For the missing-tool path, choose a **fresh cache**:

```sh
CMAKE=/missing/cmake PLASMO_AUDIO_CACHE=/fresh/scratch/cache cargo test --workspace --locked --test unavailable
```

## SDK adapter and packaging

Install the official `cargo-horizon-mod` checker for the manifest's exact build.
The catalog's current checker pin is in `horizon-rust/horizon-mods/.github/horizon-rev`.
Then:

```sh
cd mod
cargo horizon-mod check
cd ..
python3 tools/check_sdk.py --interop
python3 tools/check_sdk.py --clippy
python3 tools/package_sdk.py
```

`--interop` requires the OpenSSL command and checks RSA SPKI/PKCS#1 v1.5 exchange
with an independent implementation. The adapter script copies source and API to
`.local/` before ordinary Cargo tests, preserving the mod's registry lock.
Its source-API build is not evidence of prebuilt SDK ABI compatibility.

The package tool runs the official build/checker and the recursive source-payload
inspection, then writes `.hmod` plus SHA-256 under `dist/`. Packaging never compiles
the bundled audio archives. `.horizon/`, development targets and player files are
excluded. Keep the root workspace and `mod/Cargo.lock` separate.

## Actual signed-SDK build

Install and prepare the matching official Nightly in a disposable launcher home,
using your own Minecraft installation, then:

```sh
horizon-launcher native-build /absolute/path/to/mod --instance TEST_INSTANCE --trust-native-code --json
```

This must take the **prebuilt** route and compile only the mod. The launcher's
`native-mods.json` records build, toolchain, route and mod/library identities.
Record that result and game tests separately from source-only adapter tests.
Never publish prepared resources, game jars, compiled SDK binaries or account files.

## Registry dependency lock

The SDK's registry identities and dependency edges are immutable. A plain Cargo
resolution of API source is different from linking the prebuilt SDK. The launcher
owns the final registry/SDK reconciliation; do not update its game lock.

`tools/lock_sdk_registry.py --sdk-lock /path/to/published-sdk/Cargo.lock` resolves
an inert manifest, preserving the SDK graph while adding the mod's private
registry dependencies. It executes no submitted build scripts or proc macros.
Use it only when deliberately changing dependencies, then validate with the real
launcher build above. Never regenerate the release lock as a side effect of tests.

CPAL was removed because it introduced a conflicting native ALSA provider against
the SDK. The independent OpenAL 1.1 ABI wrapper now owns selected capture/playback
devices as well as loopback HRTF. Public API values do not cross private registry
serde/glam/RSA implementations: the bridge converts plain data at that boundary.

## Reviewable audio preparation

Complete unmodified sources and notices live under `mod/assets/plasmo_voice/audio/`.
Original Rust verifies SHA-256, confines extraction/build to an instance cache,
uses one compiler job and writes a hash receipt. A background worker does this
once; UI/render input never blocks on CMake. Failed preparations require an
explicit retry. CMake processes are cancelled on client shutdown. Cache loading
never falls back to arbitrary codecs on PATH.

Opus/RNNoise use generic C with baseline x86_64 flags. OpenAL remains a separate
LGPL shared library with its complete corresponding source available locally.
ALSA (Linux), WASAPI (Windows), null and loopback backends are enabled; PulseAudio
and PipeWire native backends are not. Physical device behavior requires separate
manual testing.

## Branding

Original artwork is in `branding/`; `python3 tools/export_branding.py` exports
sizes using ImageMagick and rsvg-convert. The package's `mod/icon.png` and catalog
icon share that artwork. In-game pixel textures are exported from the original
Rust icon design.
