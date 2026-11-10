# API adapter validation, 5 November 2026

## 0.5.4 current Nightly packaging and disabled audio, 10 November 2026

The catalog's current native-compat check rejected 0.5.3's separate installer
crate: bundle crates may not depend on each other by path. Version 0.5.4 uses
one declared `engine` crate containing protocol/audio code, build integration
and the installer binary. The trait adapter points to that declared crate at
its normal staged location; there are no path dependencies inside the bundle
crate. All registry/build dependencies already exist in Horizon's Cargo.lock.
The cmake and tar Rust crates are removed: original Rust build integration
uses the already locked cc and sha2 crates and invokes local CMake directly.
CMake extracts only the exact verified upstream archives, under OUT_DIR.

The package is checked against public Nightly **2026-11-08**, core
`fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca` and its Mixins tool
`5a64af938ec86b00d53d16a408aabc0038fb3e57`. Registered API minimum 411 is
retained. Application passes and all **522 registry package identities and
checksums** remain unchanged. Both ordinary engine and opt-in DSP checks pass;
codec/DSP loopback from the applied package passes without physical devices.
The dedicated bundle checker additionally stages and checks the trait adapter
with --locked, rather than relying only on ZIP structure validation.

Unsupported targets (including macOS/MSVC), missing CMake/Ninja/compiler and
failed audio compilation produce a generated disabled build result. The
installer exits successfully for that result, records the disabled reason and
keeps source/license files. The loader rejects all codecs, including stale
installed libraries, and Client::start refuses to create a worker. The mod list,
settings list and voice menu explain the cause in English or Russian. Install
the missing tools and rebuild to enable voice. No voice network session or
physical audio device is opened in this mode.

The missing-CMake installer and disabled-worker regressions pass locally.
CI covers ordinary Linux/Windows builds, Linux without CMake, macOS as an
unsupported target, and real bundle application/trait checking on the exact
Nightly. Each CI result is linked from the release after completion. Earlier
live UI, Windows and five-server records below retain their original scope.

## 0.5.3 source-built audio dependencies, 7 November 2026

Catalog review #305 requested removal of all six opaque native libraries. This
version replaces them with complete, byte-verified upstream source archives:
Opus 1.6.1, RNNoise 0.2 and OpenAL Soft 1.25.2. Cargo verifies their SHA-256
values and compiles shared libraries through CMake in OUT_DIR. The source build
uses no network. The installer copies only these compiler outputs, original
licenses and complete corresponding source into the instance data directory.
Installed libraries take precedence; there is no system/PATH codec fallback.

Linux: **23 engine tests**, the **two opt-in DSP tests**, and **six API 411
adapter regressions** pass. The DSP tests measure RNNoise noise reduction,
left/right HRTF positioning and independent gain (4x amplitude / 16x energy).
Engine and adapter Clippy pass with warnings denied; the optimized API 411 game
build completes in 5m48s. The exact packaged installer compiles separately. The post-build installer
passes Opus round-trip, RNNoise and HRTF loopback checks without physical
devices; a repeated installation preserves identical files and valid hashes.

Windows GNU: all three DLLs compile from the same archives. Their imports
contain only Windows/UCRT libraries, without additional GCC, C++ or pthread
runtime DLLs. The Windows installer, codec/DSP checks and repeat installation
also pass under Wine. This is not a Windows GUI or physical-device test.
The CI workflow additionally runs engine/DSP checks on an actual Windows runner
and the Linux runtime on QEMU's baseline qemu64 CPU without AVX.

The source-only bundle checker validates ZIP integrity, each upstream archive,
source member paths and executable signatures/extensions, finding **zero
prebuilt executable payloads** and **1,007 source files**. It rejects the
historical 0.5.2 bundle. Earlier UI and five-server checks below retain their
original scope; they were not repeated as a new version matrix for 0.5.3.

Build requirements are explicit: C/C++ and CMake on Linux, and Windows GNU
MinGW-w64 plus CMake and Ninja on PATH. The launcher supplies Rust/MinGW;
it does not currently provision CMake/Ninja. The mod still requires registered
Hooks/Host API 411 and has no private Game or Minecraft protocol injections.

## 0.5.2 connection indicators, 6 November 2026

Six adapter regressions pass against public core
`d52ff7a6f783a164043549f5f04645d5324d0d8a` (API 411), including captured HUD
draws for ordinary servers, real connection attempts, timeout/error, listen-only
and PTT states. All 23 engine tests pass, including encrypted UDP loopback and
a deterministic silent-server / stalled-handshake / delayed-endpoint regression.
Engine/adapter Clippy and the Windows x64 adapter cross-check pass.
Only a real handshake animates; the TCP/configuration deadline is 15 seconds.
The earlier multi-version and platform checks below retain their original scope.

Two fresh isolated Linux clients on the optimized build also passed live LAN
checks without a voice plugin: no activation icon at idle, while physical PTT
was held, or after 16 seconds. Diagnostics stayed `Waiting` with audio devices
closed. Physical Shift + RMB + wheel, both Shift keys, volume indicator expiry,
hotbar behavior, bounds and blocked targets also passed again. Test clients and
their private display were stopped. Audio was disabled for these UI checks.
[Results](https://github.com/rison1337/horizon-plasmo-voice/blob/v0.5.2/docs/validation/connection-0.5.2.json).

## 0.5.1 input adjustment

The registered API 411 build passes five adapter regressions and Clippy with
warnings denied. A separate two-client Linux LAN fixture used physical X11
input and screenshots to verify Shift + RMB + scroll, either Shift key,
reverse scroll, normal hotbar behavior without the chord, wall/target checks,
and no indicator on button holds or unchanged volume limits. The indicator
expires after 1.5 seconds even with the chord held. Audio was disabled; this
input/HUD update does not repeat or extend the 0.5.0 voice-version matrix below.

Linux Opus 1.6.1 and RNNoise 0.2 were also rebuilt for baseline x86_64 with
runtime CPU detection after the old binaries failed CI with SIGILL. The old
set reproduces SIGILL on QEMU qemu64; the new set passes Opus/RNNoise/HRTF
loopback on the same no-AVX model. CI repeats this CPU compatibility check.
The Rust engine, Windows runtimes and OpenAL binaries are unchanged.

The adapter is original Rust; the Plasmo Voice implementation and graphics were
not read. Reference behavior was observed by running released Fabric 1.21.11
with Plasmo Voice 2.1.17 in a disposable LAN world. Those UI runs use separate
Game/Launcher/XDG directories, disable self-update and use HORIZON_NO_AUDIO=1.
The saved Microsoft session was reused privately for those earlier LAN checks.

## Accepted API adjustment

Both #9556 and #9601 are merged. The mod no longer calls the helper removed by
the maintainer, `enable_older_servers`; it does not write process environment.
A player explicitly supplies HORIZON_OLDER_SERVERS=1 for older servers.
Development uses public merge ffd9ffb99554fc64433ca8a9258cf7a8ec1d5c2f without
core patches. No official hooks number has been invented or locally stamped.

## Official API registration

Horizon PR #9812 merged on 5 November 2026, registering the required extensions
as APIs 410 and 411. The release uses `hooks: 411` and public source
`d52ff7a6f783a164043549f5f04645d5324d0d8a`. The unchanged adapter compiles on
that source and all four regressions pass. The earlier UI integration checks
below retain their original source identity. A separate remote version matrix
also runs the registered API game with synthetic audio, as recorded below.

## Automated checks

- Accepted public source: adapter Clippy with warnings denied and rustfmt pass.
- Accepted public source: x86_64-pc-windows-gnu adapter cargo check passes.
- Four adapter regressions cover ordered control retry, selection blocked by
  nearer entities/blocks, posed head/helmet/crouch clearance, PTT release and
  immediate settings flush on disconnect.
- The unchanged voice engine has 22 passing tests. Earlier voice checks use
  independent protocol fixtures and UDP/AES/Opus signals; see ../VALIDATION.md.
- Portable packaging refuses the unstamped API. A pinned development package
  is an explicit separate mode. Both OS runtimes and their licenses are bundled.

## Real game integration

The initial private integration of these APIs passed all of these checks before
the maintainer's helper removal; see the local JSON reports for each run:

- Six Russian settings tabs and 13 saved flag round trips; sliders, resets,
  source mute, device lists, key rebinding and drop-downs exercise actual input.
- English: all six tab labels and stable tab bounds.
- Physical PTT press/release and visible foreground indication across eight
  screens: playing, pause, options, video, sound, inventory, mod settings, chat.
  Focus loss clears the hold. This checks input/UI with audio devices disabled,
  not actual voice transmission.
- Independent world ruler: eight abrupt turns, first-frame centering error
  <=1 pixel; distances 2–28 blocks, width error <=1 pixel and monotonic shrinking.
  The ruler is sampled above the helmet so the player's model cannot obscure it.
- Actual pixels: opaque stone hides icons; glass does not; a bottom slab leaves
  the ray clear and the top slab intersects it and hides the icon.
- Physical Shift+RMB+wheel changes player volume without switching hotbar slots;
  reverse wheel decreases it; ordinary scroll still changes the hotbar. Looking
  away or a wall prevents selecting the player. Settings are persisted.
- Disconnect and reconnect complete a fresh voice handshake: Connected, two
  players, no microphone/output device opened.

The official launcher built the API-only bundle in Performance mode; its Rust
post-build step verified Opus/RNNoise/OpenAL HRTF without physical devices.
Accepted-source repeat: the official launcher downloaded public ffd9ffb and
built in 6 min 15 s. A launch without HORIZON_OLDER_SERVERS was rejected by the
1.21.11 server; explicitly setting it enabled the normal voice handshake. The
accepted build repeated all eight PTT screens, focus loss, wheel adjustment,
block-shape occlusion, independent ruler/first-frame checks, helmet clearance
from four sides, a visible name and real crouching, plus disconnect/reconnect
with saved player levels. All passed. English tab labels and bounds are checked
on the accepted build as well. No physical sound devices were opened.

Reports live under `.local/native-api-validation/accepted/` in the project;
full builds and screenshots remain private on the SSD. The distributable source,
runtime files and JSON/TOML in the development package match the launcher-built
package byte for byte; later README/validation updates do not change its code.

## Remote version checks on the registered API

An optimized game built on d52ff7a6f783a164043549f5f04645d5324d0d8a was tested
against isolated Paper servers with released Plasmo Voice 2.1.17. Two offline
test clients used fresh Game/Launcher/XDG folders and explicitly routed virtual
audio. Synthetic 997 Hz capture passed through the actual UDP/AES/Opus path in
both directions, including PTT in the pause menu; a separate output recording
checked the received frequency and nonzero RMS. No physical audio was used.

Server-issued named/lore items, nested shulker contents, legacy map_color and
chest shift-clicks were checked against server-authoritative inventories.
An explicit goat-horn instrument component still fails on the tested
1.21.10/1.21.11 core. Horizon PR #8659 is included, but the additional upstream
fix is tracked separately in PR #10038. The voice mod does not patch these
Minecraft decoders or enable older protocols. HORIZON_OLDER_SERVERS=1 is still
required by Horizon for older servers; a launch without it was rejected.

Full per-version results and limits:
https://github.com/rison1337/horizon-plasmo-voice/blob/main/docs/SERVER_COMPATIBILITY.md

## Limits

No graphical Windows execution or physical Windows microphone/output test is
claimed. Cross-compilation checking alone does not establish device behavior.
No claim of full pixel-for-pixel or DSP identity with the Fabric implementation
is made. The portable release uses the official stamped API minimum 411.
Prepared resources, sessions and integration builds stay out of the public tree.
