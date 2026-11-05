# Plasmo Voice 0.5.0: Hooks / Host

`native-api/` is the complete public-API adapter for the existing Rust voice
engine. Its entry point is `hooks() -> Box<dyn Hooks>`; the ordinary `.hnmod`
uses `[api] path = "api"`. No injected Game modules, raw targets, private Game
fields or item-decoder replacements are used.

## Accepted Horizon changes

Both [PR #9556](https://github.com/horizon-rust/horizon/pull/9556) (final GUI
layer and current-frame camera) and [PR #9601](https://github.com/horizon-rust/horizon/pull/9601)
(client Host services and lifecycle events) were merged on 5 November 2026.
The mod also uses the existing API 408/409 packet receive/send helpers.

**The accepted version deliberately excludes `enable_older_servers`.** A mod
must not switch the experimental older-server adapters on for all players.
For Minecraft 1.21.11, explicitly add `HORIZON_OLDER_SERVERS=1` to the selected
instance's **Environment variables** field in the launcher's launch settings.
This works on Linux and Windows; then start it with Play as usual. It is not
needed for servers using Horizon's pinned Minecraft version. The mod neither
changes this setting nor mutates its process environment.

The bundle pins public Horizon commit
`ffd9ffb99554fc64433ca8a9258cf7a8ec1d5c2f`, containing both accepted PRs and the
maintainer's adjustment. The launcher's normal native build downloads this
public source; no prepared private source cache is required. Until the new API
fragments are stamped, no `hooks` number is claimed. The package uses the public
Hooks/Host API but is still pinned to this tested commit. After stamping, a
release with the actual API minimum can build with future compatible Nightlies.

## Installation

After the official API minimum is stamped, put `dist/plasmo-voice-0.5.0.hnmod`
in the selected instance's
`horizon-native-mods/`. Enable `plasmo_voice` on the Mods page, select
**Performance**, Build, then Play. For a 1.21.11 server, also set the environment
variable above. Do not extract the `.hnmod` or install the older 0.4.4 alongside it.

The one file includes Linux x86_64 and Windows x64 runtimes. Its independent
Rust post-build installer verifies and installs the required libraries for the
current system. No separate client archive or library search-path setting is
needed. Building on Linux requires a C compiler, pkg-config and ALSA/libudev
headers; the launcher obtains its Rust toolchain, and Windows' MinGW toolchain.

## Preserved functionality

The unchanged voice engine provides Opus, RNNoise, spatial/HRTF output,
six settings tabs, input/output selection, activation modes and bindings,
PTT over menus, microphone indication, player/source overlays, model-head icons
with helmet clearance, visual clipping, per-player volume multiplied by the
proximity level, wheel/chord control, distance visualization, mute states,
and server-controlled activations/sources. English and Russian are included.
The engine runs on its worker; the adapter uses owned client snapshots.
The additional world-canvas PR #8918 is not required: head icons use the current
camera and visual-shape clipping, including partial blocks and glass.

## Development

Stage the unchanged public core and the adapter for local checks:

```sh
python3 tools/prepare_native_api.py --horizon /path/to/horizon
```

Build a development check against the public accepted commit (no core patch):

```sh
python3 tools/package_native_api.py --source .local/native-api-accepted --pinned
```

This creates `dist/plasmo-voice-0.5.0-pinned-check.hnmod`, not the portable
release. After the official release process stamps the accepted API fragments:

```sh
python3 tools/package_native_api.py --source /path/to/stamped/horizon \
  --hooks OFFICIAL_MINIMUM_API_NUMBER
```

The tool rejects release mode until every required hook and helper is in the
stamped catalog and the API level matches the tested source. Never guess a
number or stamp a private number into a distributed package. Adding this mod
to the public catalog remains a separate publication step.

The older `0.5.0-preview.1-native-api` package used unpublished integration
source and is superseded. Previous local source-injection experiments are not part of this API-only
source repository. No old core patch is applied by 0.5.0.
See [VALIDATION.md](VALIDATION.md) for what has actually been checked.
