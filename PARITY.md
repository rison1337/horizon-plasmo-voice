# Compatibility and feature coverage

This is an independent protocol-compatible implementation, observed against
released Plasmo Voice **2.1.17** on Fabric **1.21.11**. Original implementation
sources and original graphic assets were not read. Public serializers and a
permissively licensed protocol reference supplied independent wire fixtures.
It is not a claim of identical pixels, DSP, or every server add-on's behavior.

| Area | Implemented behavior and evidence |
| --- | --- |
| Transport | RSA handshake, UDP, AES-CBC, Opus; independent wire fixtures and live original-client interoperability |
| Audio worker | Capture, network and playback off the game thread; bounded queues, packet reordering, loss recovery and adaptive jitter |
| Spatial sound | Distance attenuation, camera-relative stereo, directional sources, occlusion/low-pass, optional OpenAL Soft HRTF |
| Input | PTT, voice activation, server activations, key chords, focus-loss release and hotkey capture suppression |
| Menus | Six tabs, device choices, preview, thresholds, gains, flags, drop-downs, sliders, resets and persistent bindings |
| Volumes | Master, source/channel, and per-player controls; independent multiplicative player gain |
| Wheel control | RMB + wheel including Shift; preserves hotbar selection, respects nearer entities and walls |
| Own indicator | Held PTT, transmission, mute and connection states drawn over normal interfaces |
| World icons | Current-frame projection, fixed world size, interpolated head centering, name/helmet clearance and shape-aware visibility |
| Overlays | Names/skins, positions/styles, player/source states and server-provided PNG icons |
| Server data | Activation/channel registries, group membership, player/entity/static/direct sources, distances and permissions |
| Translations | English and Russian |

## Scope

Group, radio and music behavior depends on what the server's add-ons expose.
Arbitrary Java client extensions, ReplayMod integration, and a separate Sound
Physics implementation are not provided. The mod does not implement Minecraft
version adapters: those are Horizon services.

The accepted API port removes private Game injections and item-decoder patches.
The existing Rust engine is retained. API registration is still required before
claiming future-Nightly portability. Detailed run results and platform limits:
[VALIDATION.md](VALIDATION.md).
