# API adapter validation, 5 November 2026

## 0.5.1 input adjustment

The registered API 411 build passes five adapter regressions and Clippy with
warnings denied. A separate two-client Linux LAN fixture used physical X11
input and screenshots to verify Shift + RMB + scroll, either Shift key,
reverse scroll, normal hotbar behavior without the chord, wall/target checks,
and no indicator on button holds or unchanged volume limits. The indicator
expires after 1.5 seconds even with the chord held. Audio was disabled; this
input/HUD update does not repeat or extend the 0.5.0 voice-version matrix below.

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
