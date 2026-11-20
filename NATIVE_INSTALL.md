# Installation — 0.6.0

This release uses a source-only **`.hmod`**, Horizon SDK API 0.2 and Launcher
0.2.85 or newer. It targets Nightly **2026-11-19**, game build
`f00125f61f794aaa3d4e950d301f353b35b6ee3d`, Minecraft resources 26.3.
Do not use the old `.hnmod` release with the SDK runtime.

## Install through the launcher

1. Install the matching official Nightly and prepare resources from your own
   Minecraft installation. Select it for the instance.
2. Open Mods, choose **Add from GitHub**, and use
   `https://github.com/rison1337/horizon-plasmo-voice/releases/tag/v0.6.0`.
3. Read and accept Horizon's native-code notice. Build native mods, then Play.

For a manual install, download `plasmo-voice-0.6.0.hmod` from that release and
place it in the instance's `horizon-native-mods/` before building. Keep only one
version enabled. No installer, custom game archive or manual DLL copy is needed.
The launcher compiles only mod code against the signed prebuilt SDK.

The CLI equivalent, after installing/preparing the official Nightly, is:

```sh
horizon-launcher mods add-github https://github.com/rison1337/horizon-plasmo-voice/releases/tag/v0.6.0 --instance YOUR_INSTANCE --trust-native-code
horizon-launcher native-build --instance YOUR_INSTANCE --trust-native-code
```

A GitHub release is installable independently of catalog review. It appears in
the Horizon catalog only after maintainers accept the submission.

## Audio prerequisites

Audio preparation runs offline in a background worker on first use. It builds
Opus, RNNoise and OpenAL Soft from the complete verified sources in the package,
then checks and reuses `.plasmo_voice/audio/` in this instance. Closing the game
cancels its compiler processes. These are ordinary native compiler commands,
not downloaded executables; the first preparation can take several minutes.

| Platform | Requirements |
| --- | --- |
| Linux x86_64 GNU | C/C++, CMake, pkg-config, ALSA development headers |
| Windows x64 GNU | MinGW-w64 C/C++, CMake and Ninja on the launcher's PATH |
| macOS / Windows MSVC / other targets | Voice disabled with a translated explanation |

On Debian/Ubuntu:

```sh
sudo apt-get install build-essential cmake pkg-config libasound2-dev
```

On Windows, the launcher supplies its Rust/MinGW toolchain for the SDK build.
CMake, Ninja and the C/C++ toolchain must also be reachable by the running game's
PATH for first audio preparation. Restart the launcher after changing PATH.

Missing tools or failed audio preparation do not prevent the game/mod from
building. Open **V** for the explanation; after installing tools choose
**Reconnect voice** to retry. The detailed log is `audio-build.log` under
`.plasmo_voice/audio/<target-and-hash>/`. No system codec fallback is used.

## First use

Press **V**, select input/output devices and choose activation mode. Hold
**Left Alt** for PTT; **M** toggles microphone mute. Change bindings in Hotkeys.
Saved custom bindings take precedence over registered game-control defaults.
The local microphone preview is not sent to the server.

PTT remains active over interfaces. The indicator distinguishes held activation,
actual transmission and muted states; a held key does not override mute.
Aim at another player and hold **Shift + RMB** while scrolling to change their
volume. The hint appears after an actual change and fades after 1.5 seconds.
Individual player, source and master gains multiply.

Minecraft server-version support belongs to Horizon. This mod neither enables
old adapters nor alters inventory decoding. The historical server matrix is in
[docs/SERVER_COMPATIBILITY.md](docs/SERVER_COMPATIBILITY.md); its old environment
settings are not installation requirements for the new SDK release.

## Data and permissions

The mod uses selected audio devices, communicates with the joined server's
advertised voice endpoint and stores instance-local settings/audio cache.
It adds no telemetry service. Native mods run with the game's account permissions
and use Horizon's normal trust flow. Review the source before installation.
