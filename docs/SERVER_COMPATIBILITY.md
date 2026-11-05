# Server compatibility checks

Tested on **5 November 2026** with the unchanged 0.5.0 voice adapter and engine,
Horizon public source `d52ff7a6f783a164043549f5f04645d5324d0d8a`, native API
**411**, and released **Plasmo Voice 2.1.17** on Paper.

| Minecraft server | Paper build | Game and voice handshake | Audio both directions, including pause-menu PTT | Baseline inventory and chest | Explicit horn instrument |
| --- | --- | --- | --- | --- | --- |
| 1.21.10 | 130 | Passed | Passed | Passed | Upstream failure |
| 1.21.11 | 132 | Passed | Passed | Passed | Upstream failure |
| 26.1.2 | 74 | Passed | Passed | Passed | Passed |
| 26.2 | 135 | Passed | Passed | Passed | Passed |
| 26.3 | 242 | Passed | Passed | Passed | Passed |

[Machine-readable results](validation/multiversion-2026-11-05.json) include
the measured output frequency and RMS for each direction. These are specific
integration cases, not exhaustive Minecraft protocol certification.

## What ran

Two separately running Horizon clients joined a remote, whitelisted, offline
test server over a public TCP/UDP endpoint. Each version had its own world and
configuration; only one server ran at a time. Java 21 ran the 1.21.x servers and
Java 25 ran the 26.x servers. Test runs used fresh Game/Launcher/XDG directories.

Synthetic **997 Hz** input was routed through named virtual capture devices to
the real capture, encrypted UDP, Opus and output path. A separate recording of
the virtual output measured **996.71–996.79 Hz** and nonzero RMS in all ten
directions. The reverse direction held physical Left Alt while the sender's
pause menu stayed open. No physical microphone or speaker was used.

Server commands issued seven named diamonds with lore, a shulker containing
three diamonds, and two more diamonds with `map_color` on versions that still
provide that component. Client counts and retained name/container components
were checked. Thirteen emeralds were shift-clicked from a chest to the player
and back; both inventories were checked against server-authoritative data.
These baseline cases produced no client decode errors. A separate goat-horn
probe followed them.

## The inventory fix and remaining issue

[Horizon PR #8659](https://github.com/horizon-rust/horizon/pull/8659) is already
an ancestor of the tested core. It handles older item component layouts,
including obsolete `map_color` and nested stacks. This mod carries no duplicate
inventory patch.

However, giving `goat_horn[instrument="minecraft:ponder_goat_horn"]` succeeds
on the 1.21.10/1.21.11 server while the item fails to appear in Horizon. The
client records item-packet translation failures. Some failures occur before
the decoder counter and therefore are not reflected in `status.server.decode_errors`.
The same item arrives on 26.1.2, 26.2 and 26.3.

The additional core fix is tracked in
[Horizon PR #10038](https://github.com/horizon-rust/horizon/pull/10038).
Its acceptance and the next Nightly are separate from the voice release.
Until a core with that fix is tested, these older-server results retain this
limitation. The voice mod does not change Minecraft's item or protocol adapters.

## The older-server opt-in is still required

An actual launch without `HORIZON_OLDER_SERVERS=1` was rejected by the 1.21.10
server as an outdated-server connection. All four older-version tests used
that explicit Horizon opt-in. Both 26.3 clients were restarted without it and
joined successfully. The inventory fix does not remove this gate.

Set `HORIZON_OLDER_SERVERS=1` in the instance's launch environment for older
versions supported by Horizon. This belongs to Horizon's launch configuration;
the voice mod does not set it or contain an extra protocol-version switch.
The `.hnmod` manifest's `game = "26.3"` describes its Horizon build/resources,
not a promise that only 26.3 voice servers can be contacted.

## Limits

The matrix covers Linux x86_64, Paper and the exact builds above. Shared wire
protocols do not count as separate live tests of other patch releases. Earlier
Fabric 1.21.11 UI/reference checks are recorded in [Validation](../VALIDATION.md).
Windows cross-checking does not establish Windows GUI or physical audio-device
operation. Human speech quality, every item component and controlled latency
or packet-loss scenarios remain untested.

Disposable local clients, display and virtual audio devices, and the remote
test server were stopped after validation. Prepared game resources, server
binaries, worlds, addresses, credentials and raw logs remain outside Git.
