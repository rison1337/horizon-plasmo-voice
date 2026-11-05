# Publishing checklist and prepared catalog entry

The owner authorized public source and a catalog submission after Horizon
PR #9812 merged. Version 0.5.1 uses the official native API minimum **411**.
The files under `publishing/horizon-mods/` are the submission material. Catalog
acceptance is a separate maintainer decision; a GitHub Release alone does not
make the mod appear in Horizon Launcher.

The source is public and the normal
[v0.5.1 release](https://github.com/rison1337/horizon-plasmo-voice/releases/tag/v0.5.1)
is published. [Catalog PR #305](https://github.com/horizon-rust/horizon-mods/pull/305)
contains the entry, icon and local check results. It requests the reviewer's
required staging upload; neither the release nor this submission claims catalog
acceptance.

1. Package the registered API using its actual minimum. Run the adapter checks,
   engine tests and `cargo horizon-mod check`.
2. Review the source, README, license, runtime notices and release notes. Make
   the repository public only when its owner chooses to publish it.
3. Publish a normal GitHub Release with the immutable `.hnmod` asset. Keep the
   release version consistent with both manifests. Never replace published bytes.
4. Submit the project through https://playhorizon.dev/mods/submit, or open a PR
   in https://github.com/horizon-rust/horizon-mods using the prepared entry and
   `icons/plasmo_voice.png`. The listing type is `native-mod`, id `plasmo_voice`.
5. Registry review and acceptance make the project appear on the site and in
   the launcher's Horizon catalog. Creating this repository alone does not.

A public catalog listing needs public source. Native mods also have Horizon's
trust/build prompts. Registration of the voice mod and registration of the
Horizon hooks are separate steps.

The entry's icon is a square PNG under 256 KiB. Its name, summary, authors and
license are mirrored in `horizon-registry.toml`. Published version entries use
the actual artifact hash and date. No signature or download count is invented.

The catalog currently publishes explicit `[[versions]]` entries. A `repo` field
alone does not populate its existing index workflow. A manual submission also
needs a reviewer to copy the public release asset to the catalog's `staging`
release as `<sha256>.hnmod`, as its PR template specifies. The submission must
provide the source release, exact SHA-256 and local registry-check results;
catalog access is not implied by owning the mod's separate source repository.

## GitHub artwork

- README/project logo: `assets/icon-256.png`.
- Launcher catalog icon: `publishing/horizon-mods/icons/plasmo_voice.png`.
- Social preview: `assets/github-social.png`, 1280 × 640.

GitHub social-preview artwork is uploaded through repository Settings → General
→ Social preview. The README already references the shared logo and banner.
GitHub permits a first social-preview upload on a public repository; a new private
repository cannot be assigned its first custom preview yet. Keep the supplied
image for publication instead of changing this repository's visibility.
Reference: https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview

## Release notes

**0.5.1 — player-volume controls**

- Requires Shift + the action key (RMB by default) + scroll; either Shift key works.
- Displays volume only after an actual change, and expires after 1.5 seconds
  even if the buttons remain held. Normal scrolling retains hotbar behavior.
- Five adapter regressions, Clippy, and an isolated two-client physical-input/HUD
  check pass. The unchanged engine retains the documented 0.5.0 voice matrix.

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
