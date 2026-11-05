# Publishing checklist and prepared catalog entry

This repository is private development work. It is not listed in the Horizon
catalog and has no public release. The files under `publishing/horizon-mods/`
are prepared submission material, not a submitted or approved listing.

1. Finish the official hooks registration and package using its actual API
   minimum. Run the adapter checks, engine tests and `cargo horizon-mod check`.
2. Review the source, README, license, runtime notices and release notes. Make
   the repository public only when its owner chooses to publish it.
3. Publish a normal GitHub Release with the immutable `.hnmod` asset. Keep the
   release version consistent with both manifests. Never replace published bytes.
4. Submit the project through https://playhorizon.dev/mods/submit, or open a PR
   in https://github.com/horizon-rust/horizon-mods using the prepared entry and
   `icons/plasmo_voice.png`. The listing type is `native-mod`, id `plasmo_voice`.
5. Registry review and acceptance make the project appear on the site and in
   the launcher's Horizon catalog. Creating this repository alone does not.

A private repository cannot serve as the source of a public catalog listing.
Native mods also have Horizon's trust/build prompts. Registration of the
voice mod and registration of the Horizon hooks are separate steps.

The entry's icon is a square PNG under 256 KiB. Its name, summary, authors and
license are mirrored in `horizon-registry.toml`. The template deliberately has
no fabricated API level, release URL, file hash, signature or download count.

## GitHub artwork

- README/project logo: `assets/icon-256.png`.
- Launcher catalog icon: `publishing/horizon-mods/icons/plasmo_voice.png`.
- Social preview: `assets/github-social.png`, 1280 × 640.

GitHub social-preview artwork is uploaded through repository Settings → General
→ Social preview. The README already references the shared logo and banner.

## Release notes draft

**0.5.0 — native Hooks/Host client**

- Replaces private Game injections with public Hooks/Host callbacks and services.
- Preserves proximity audio, activation modes, six settings tabs and bindings.
- Keeps PTT and its indicator active over normal interfaces.
- Uses current-frame head poses with helmet/name clearance and block visibility.
- Provides per-player gain and RMB/wheel control alongside source/master levels.
- Ships Linux x86_64 and Windows x64 codec/DSP runtimes in one `.hnmod`.
- Adds a shared original Minecraft-inspired pixel microphone identity.

State the actual official API minimum and tested Horizon commit in the final
release notes. Retain the documented Windows GUI/device-testing limitation.
