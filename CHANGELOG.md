# Changelog

## 0.5.1 — 2026-11-05

- Requires Shift + the action key (RMB by default) + scroll to adjust a targeted
  player's volume; either Shift key works.
- Shows the volume indicator only when a scroll actually changes the volume.
  Holding RMB no longer displays or refreshes it.
- Keeps the hotbar still when adjustment reaches the 0% or 200% limit, without
  showing a new change indicator.
- Rebuilds the Linux Opus/RNNoise runtime for baseline x86_64 with runtime CPU
  detection, fixing SIGILL on processors missing the old binaries' instructions.

## 0.5.0 — 2026-11-05

- Moves the native client to public Horizon Hooks/Host services.
- Removes private Game injections and Minecraft item/older-protocol patches.
- Preserves the existing Rust voice engine and six-tab English/Russian UI.
- Supports PTT and foreground indication over menus, focus-loss release,
  individual player gain, wheel controls, and stable world-space head icons.
- Bundles Linux x86_64 and Windows x64 runtimes with a checked post-build step.
- Adds an original pixel microphone identity for GitHub and the launcher catalog.
- Records two-way synthetic voice, pause-menu PTT and inventory/chest checks on
  Paper 1.21.10, 1.21.11, 26.1.2, 26.2 and 26.3, including the remaining upstream
  1.21.x instrument-component limitation.

Requires Horizon native API **411 or newer**, registered by Horizon PR #9812.
Tested release source: `d52ff7a6f783a164043549f5f04645d5324d0d8a`.
The `.hnmod` includes a validation note under a name accepted by Horizon's
native bundle validator. Previous explicitly source-pinned test packages remain
development artifacts.
