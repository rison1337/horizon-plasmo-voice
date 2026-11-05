# Installation

## Current status

Download `plasmo-voice-0.5.1.hnmod` from
[Releases](https://github.com/rison1337/horizon-plasmo-voice/releases/tag/v0.5.1).
Use a Horizon Nightly with native API **411 or newer**. Both required extensions
are officially registered by [Horizon PR #9812](https://github.com/horizon-rust/horizon/pull/9812).
The release records the tested public Horizon commit
`d52ff7a6f783a164043549f5f04645d5324d0d8a` and declares `hooks: 411`, so it can
also build on compatible later Nightlies.

See [Development](docs/DEVELOPMENT.md) to build the package yourself.

## Add the mod to an instance

1. Open the selected Horizon instance's folder.
2. Put the single `.hnmod` file in `horizon-native-mods/`.
3. On the launcher's **Mods** page, enable `plasmo_voice`.
4. Select **Performance** and press **Build**. Accept the launcher's native-code
   trust prompt when you choose to install the mod.
5. Start that instance with **Play**.

Keep only one version of `plasmo_voice` enabled. No client archive, launcher
script, manual DLL copy, or library search-path setting is required. The package
contains runtimes for Linux x86_64 and Windows x64; its Rust post-build step
installs the current system's files and checks their hashes and codec/DSP APIs.

## Server versions

The mod handles the Plasmo Voice protocol. Minecraft protocol translation is
Horizon functionality and remains outside this mod.

For older servers supported by Horizon, explicitly add this to the instance's launch
**Environment variables** field, on either Linux or Windows:

```text
HORIZON_OLDER_SERVERS=1
```

Save the instance's launch settings and use Play normally. This opt-in is not
needed for servers using Horizon's pinned Minecraft version. The mod does not
set it automatically. Horizon requires your own prepared Minecraft **26.3**
resources even when you join an older server.

The accepted inventory fix is already in Horizon; no item-decoder patch belongs
in this voice mod. Older protocols remain experimental. In particular, the
tested core still cannot receive an explicitly configured goat-horn `instrument`
component from 1.21.10/1.21.11 servers. See the
[version matrix](docs/SERVER_COMPATIBILITY.md) for the tested cases and upstream fix.

## Build requirements

| Platform | Requirements and current verification |
| --- | --- |
| Linux x86_64 | GLIBC 2.29+, C compiler, pkg-config, ALSA/libudev development headers; live game tested |
| Windows x64 | Launcher-provided Rust/MinGW toolchain; cross-check passed, real GUI/devices untested |

Internet access is needed for the launcher's first source/toolchain build.
The voice libraries are already bundled. Use Performance for playing; unoptimized
builds are intended for development and can have much lower frame rates.

## First use

Press **V**, choose your microphone and output device, and select an activation
mode. Hold **Left Alt** for PTT or enable voice activation. **M** toggles your
microphone mute. Saved custom bindings take precedence over defaults.

Use the local microphone preview to check input. Nothing from this preview is
sent to the server. The overlay position and size are configurable. A held-PTT
indicator and active-transmission indication represent different states; a
muted microphone does not transmit just because the key is held.

Aim at another player, hold **Shift + RMB**, and scroll to change their individual
volume. Either Shift key works. The indicator appears only after an actual
change and fades after 1.5 seconds; holding the buttons alone does not show it.
Ordinary scrolling keeps its normal hotbar behavior. Individual, source and
master gains multiply.

## Privacy

The mod communicates with the connected server's advertised voice endpoint and
uses the selected audio devices. Its settings are per Horizon instance. Native
mods run with the game's normal account permissions; Horizon's standard trust
prompt applies. The voice mod adds no telemetry service.
