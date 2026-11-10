# Validation

Each record names its version and tested core. Completed checks below do not
claim that every platform or physical audio device has been tested.

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

All **seven adapter regressions** pass locally, and the new disabled-adapter
regression also passes in an explicit missing-CMake build. Local checks cover
the disabled worker/installer, failed CMake commands, and recovery after tool
installation without cleaning or changing the environment.
[Source CI](https://github.com/rison1337/horizon-plasmo-voice/actions/runs/42878041030)
covers ordinary Linux/Windows builds, Linux without CMake, macOS as an
unsupported target, and real bundle application/trait checking on the exact
Nightly. Windows builds use the native resource compiler even with an MSVC
Rust host and GNU target. All four CI jobs pass. Earlier live UI, Windows and
five-server records below retain their original scope.

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
[CI run 42374283485](https://github.com/rison1337/horizon-plasmo-voice/actions/runs/42374283485)
passes on Linux and an actual Windows runner, including engine/DSP checks and
the Linux runtime on QEMU's baseline qemu64 CPU without AVX.

The source-only bundle checker validates ZIP integrity, each upstream archive,
source member paths and executable signatures/extensions, finding **zero
prebuilt executable payloads** and **1,007 source files**. It rejects the
historical 0.5.2 bundle. Earlier UI and five-server checks below retain their
original scope; they were not repeated as a new version matrix for 0.5.3.

Build requirements are explicit: C/C++ and CMake on Linux, and Windows GNU
MinGW-w64 plus CMake and Ninja on PATH. The launcher supplies Rust/MinGW;
it does not currently provision CMake/Ninja. The mod still requires registered
Hooks/Host API 411 and has no private Game or Minecraft protocol injections.

## 0.5.2 connection indicators

Six adapter regressions pass on registered API 411 core
`d52ff7a6f783a164043549f5f04645d5324d0d8a`. The added rendering regression
checks that a server with no voice handshake paints no activation icon, even
while PTT/mute is held; actual connection, timeout/error, listen-only, idle and
held-PTT states paint the appropriate distinct indicators.
Engine and adapter Clippy pass with warnings denied. The Windows x64 adapter
cross-check also passes; this does not test Windows GUI or audio devices.

All **23 engine tests** pass, including the existing real UDP/encrypted audio
loopback. The new deterministic session regression advances the network clock
to check silent servers, the TCP handshake's 15-second deadline before any UDP
socket exists, no repeated timeout events, delayed voice endpoints, and cleanup
when UDP negotiation stalls. Ordinary servers remain idle and can still accept
later voice handshakes instead of caching a permanent absence verdict.

The 0.5.0 five-version matrix and the 0.5.1 CPU/input checks below remain
historical records. This update changes connection state and HUD behavior; it
does not claim another five-server matrix or physical Windows audio testing.

Two fresh isolated Linux clients on the optimized build also passed live LAN
checks without a voice plugin: no activation icon at idle, while physical PTT
was held, or after 16 seconds. Diagnostics stayed `Waiting` with audio devices
closed. Physical Shift + RMB + wheel, both Shift keys, volume indicator expiry,
hotbar behavior, bounds and blocked targets also passed again. Test clients and
their private display were stopped. Audio was disabled for these UI checks.
[Results](https://github.com/rison1337/horizon-plasmo-voice/blob/v0.5.2/docs/validation/connection-0.5.2.json).

## 0.5.1 player-volume controls

The adapter now requires Shift + the configured action key (RMB by default) +
scroll. It creates or refreshes the player-volume indicator only when the value
actually changes, rather than every frame while RMB is held.

On the same registered API 411 core, **five adapter regressions** pass and
Clippy passes with warnings denied. The added regression covers ordinary,
RMB-only and Shift-only scrolling, either Shift key, a real change, the volume
limit and a blocking wall.

Two fresh isolated Linux clients also exercised physical X11 button/key/wheel
input in a LAN world with audio disabled. Screenshot checks found no volume
bar on RMB or Shift+RMB alone, found it after changes, and verified expiration
after 1.5 seconds while those buttons remained held. Both Shift keys and reverse
scroll work. Adjustment preserves the hotbar, including at 200%; scrolling at
the limit does not show a new indicator. Ordinary/RMB-only/Shift-only scrolling
retains normal hotbar behavior. Looking away or a wall prevents adjustment.
The fixture clients and display were stopped; the user's running game was not
used for these checks. Private report: `.local/volume-051-report.json`.

The detailed audio/version matrix below remains the completed **0.5.0** result;
the engine and networking Rust code are unchanged by 0.5.1. A CI SIGILL exposed
an excessive CPU requirement in the previous Linux codec binaries. Opus 1.6.1
and RNNoise 0.2 were rebuilt with `-O2 -march=x86-64 -mtune=generic`, runtime CPU
detection, and a GLIBC symbol floor no newer than 2.29. Windows and OpenAL
binaries are unchanged. The old libraries reproduce SIGILL on QEMU `qemu64`;
the new libraries pass Opus encode/decode, RNNoise and HRTF loopback on that
same CPU model with a private Ubuntu userspace. CI now repeats that no-AVX
runtime check. This is a codec/DSP compatibility check, not another live voice
version matrix or physical-device test.

## Source and package

- Both Horizon extensions are merged: [#9556](https://github.com/horizon-rust/horizon/pull/9556)
  and [#9601](https://github.com/horizon-rust/horizon/pull/9601).
- Tested public source: `ffd9ffb99554fc64433ca8a9258cf7a8ec1d5c2f`.
- Official Launcher **0.2.41** downloaded that source and built the ordinary
  API-only bundle in **Performance** mode in **6 min 15 s**.
- No private source cache or core patch was needed. The mod does not call the
  removed older-server enable helper and has no Game injections.
- The runtime installer verified bundled Opus, RNNoise and HRTF without opening
  physical audio devices and installed nine Linux runtime files.
- The release packager rejects unregistered hooks and uncommitted local API
  stamping. A separate `--pinned` mode is available for development checks.

## Release API registration

[Horizon PR #9812](https://github.com/horizon-rust/horizon/pull/9812) is merged.
Version 0.5.0 declares the official minimum `hooks: 411`, tested against public
source `d52ff7a6f783a164043549f5f04645d5324d0d8a`. All four unchanged adapter
regressions pass on that exact source. The remote version matrix below runs
the optimized game on that registration merge. Earlier UI and Windows checks
retain their original scope. The runtime code and payloads are unchanged.

## Automated checks

| Check | Result |
| --- | --- |
| Engine tests | 22 passed |
| Adapter regressions | 4 passed |
| Accepted-source adapter Clippy | Passed with warnings denied |
| Adapter formatting | Passed |
| Windows x64 adapter cross-check | Passed (`x86_64-pc-windows-gnu`) |
| Package structure and both runtime payloads | Passed |

The standard engine suite passes 22 tests; two opt-in DSP tests are ignored
by default. The separate installed-runtime checks above exercise codec,
RNNoise and HRTF APIs without physical devices.

The adapter regressions cover ordered control retries during reconfiguration,
volume targeting blocked by nearer entities/blocks, posed helmet/crouch
clearance, GUI PTT release and immediate settings flush on disconnect.

## Actual Linux game checks

An isolated Fabric 1.21.11 LAN with released Plasmo Voice 2.1.17 supplied the
reference. Tests used fresh Game/Launcher/XDG directories, disabled launcher
self-update, and reused an existing Microsoft session privately.

- **Explicit Horizon opt-in:** 1.21.11 rejects the connection without its older
  server setting; supplying `HORIZON_OLDER_SERVERS=1` enables the voice handshake.
- **PTT and foreground indication:** physical press/release in playing, pause,
  options, video, sound, inventory, mod settings and chat. Focus loss clears hold.
- **English UI:** all six tab labels and stable header bounds. The original API
  integration also passed a full Russian UI check with 13 flag round trips,
  sliders, resets, devices, key rebinding and drop-downs.
- **Player volume:** physical Shift+RMB+wheel raises/lowers saved levels, preserves
  the hotbar, leaves ordinary scroll working, and ignores a player behind a wall.
- **First-frame camera:** eight abrupt turns; independent world-ruler centering
  error at most **1 px**. Distance **2–28 blocks**, width error at most **1 px**
  and monotonic perspective shrinking for the **0.32-block** icon.
- **Armor and pose:** **0 px** center error from four sides and crouching.
  Helmet gap **21 px**, independently estimated at **0.212 blocks**; a visible
  name increases the gap to **46 px**.
- **Visibility:** stone and an intersecting top slab hide icons; glass and a
  non-intersecting bottom slab preserve them.
- **Lifecycle:** disconnect/reconnect completes a new handshake and keeps saved
  player levels.

The earlier API-port UI checks used `HORIZON_NO_AUDIO=1`. They prove input/UI and
network behavior, not physical microphone or speaker operation. Earlier engine
interoperability checks used artificial signals through isolated virtual audio:
real UDP/AES/Opus in both directions, measured gain multiplication, RNNoise and
HRTF checks, and independent protocol fixtures.

## Remote server version matrix

The registered API build was also run against isolated Paper servers with
released Plasmo Voice **2.1.17**, one server at a time. Two independently running
Horizon clients joined over the public TCP/UDP endpoint with offline test-only
identities. Each client had a fresh Game/Launcher/XDG directory.

Audio was explicitly routed to named virtual ALSA/Pulse devices. Synthetic
**997 Hz** input traversed the actual capture, UDP/AES/Opus and output path in
both directions; an independent recording of the receiver's output checked its
frequency and nonzero RMS. The reverse direction held physical PTT while the
sender's pause menu remained open. Physical microphones and speakers were not used.

The inventory checks used server-issued named/lore items, nested shulker
contents, an obsolete `map_color` component where supported, and shift-clicks
in both directions between a real chest and the player. Client counts were
checked against server-authoritative entity and block data. A separate explicit
goat-horn `instrument` probe records the remaining upstream issue rather than
treating baseline item checks as complete protocol coverage.

See [Server compatibility](docs/SERVER_COMPATIBILITY.md) for the complete
version matrix, pinned Paper builds, measured results and known limitation.

## Limits and reproducibility

Actual Windows GUI/device execution, physical microphone quality, human speech,
and controlled internet-latency measurements have not been established. Do not infer them from
cross-checking alone. Pixel-for-pixel UI or bit-identical DSP is not claimed.

Unit tests and adapter staging commands are in [Development](docs/DEVELOPMENT.md).
Local integration reports remain under `.local/native-api-validation/accepted/`;
prepared Minecraft resources, accounts, sessions, raw logs and caches are
intentionally excluded from Git. The disposable game/reference/display
processes used for the earlier UI validation were stopped.
