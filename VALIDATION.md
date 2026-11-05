# Validation

Recorded on **5 November 2026** for the 0.5.0 API adapter. These are completed
checks, not a claim that every platform or audio device has been tested.

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
