# Installation

## Current status

Version 0.5.0 is a development preview. Its public Hooks/Host integration is
implemented and both required Horizon PRs are merged. A portable release needs
the official API minimum, which has not yet been registered. The available
source-built check package is named `plasmo-voice-0.5.0-pinned-check.hnmod`.
It uses public Horizon source `ffd9ffb99554fc64433ca8a9258cf7a8ec1d5c2f`.

See [Development](docs/DEVELOPMENT.md) to produce it yourself. No public release
asset is available yet. Do not treat the source check as a guarantee for every
future Nightly.

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

For a Minecraft **1.21.11** server, explicitly add this to the instance's launch
**Environment variables** field, on either Linux or Windows:

```text
HORIZON_OLDER_SERVERS=1
```

Save the instance's launch settings and use Play normally. This opt-in is not
needed for servers using Horizon's pinned Minecraft version. The mod does not
set it automatically. Horizon requires your own prepared Minecraft **26.3**
resources even when you join an older server.

## Build requirements

| Platform | Requirements and current verification |
| --- | --- |
| Linux x86_64 | C compiler, pkg-config, ALSA/libudev development headers; live game tested |
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

Aim at another player, hold **RMB**, and scroll to change their individual
volume. Shift is supported. The normal hotbar wheel action remains available
when this adjustment is not active. Individual, source and master gains multiply.

## Privacy

The mod communicates with the connected server's advertised voice endpoint and
uses the selected audio devices. Its settings are per Horizon instance. Native
mods run with the game's normal account permissions; Horizon's standard trust
prompt applies. The voice mod adds no telemetry service.
