Protocol interoperability reference: Maks-gaming/mineflayer-plasmovoice,
commit 1df7e0456c814ad5e1b37da9a52469dd5fff3bb1 (MIT).
https://github.com/Maks-gaming/mineflayer-plasmovoice/tree/1df7e0456c814ad5e1b37da9a52469dd5fff3bb1

The Rust protocol implementation adapts its packet layouts and encryption framing.
Its JavaScript runtime, audio dependencies and implementation bugs are not included.
Original Plasmo Voice implementation sources were not consulted.

The MIT License (MIT)

Copyright (c) 2023 Maksim Klimenko

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

Additional interoperability and behavior references (no implementation copied):
- Plasmo Voice 2.1.17 released public protocol serializers and public API declarations;
  released Fabric 1.21.11 client observed in an isolated local world.
  https://plasmovoice.com/docs/new-in-2xx/
- Opus public C API: https://opus-codec.org/docs/
- RNNoise public C API: https://gitlab.xiph.org/xiph/rnnoise
- OpenAL Soft published extension specifications:
  https://openal-soft.org/openal-extensions/SOFT_loopback.txt
  https://openal-soft.org/openal-extensions/SOFT_HRTF.txt
  https://openal-soft.org/openal-extensions/EXT_thread_local_context.txt
Version 0.6.0 packages complete unmodified audio sources under
mod/assets/plasmo_voice/audio/. sources.json records original URLs, versions,
licenses and verified SHA-256 values. No precompiled executable audio payload
is included. First-use preparation verifies and compiles them offline in the
instance's .plasmo_voice/audio/ cache, retaining sources and notices.

- Opus 1.6.1: BSD-3-Clause; COPYING in its release archive.
- RNNoise 0.2: BSD-3-Clause; COPYING in its release archive.
- OpenAL Soft 1.25.2: LGPL-2.0-or-later; COPYING and individual upstream notices
  are kept in its complete archive. It is a separate shared library used only
  through published OpenAL/ALC APIs, including capture/playback, loopback,
  HRTF and thread-local contexts. Users can rebuild or replace that shared
  library. Original implementation sources were not read or copied into Rust.

Rust registry dependencies retain their upstream licenses; exact versions and
checksums are in mod/Cargo.lock. RSA, image decoding and serialization remain
private to this mod. Horizon's published SDK API is linked through its designated
mod_api boundary, not redistributed in this source package. No SDK implementation
is copied into the independent voice code. UI artwork remains original.

Earlier immutable 0.5.x releases retain their original notices and behavior.
Historical tests and the legacy 0.4.x attribution below do not describe a core
patch in the current SDK mod.

Historical attribution (local 0.4.x only; not included in the 0.5.0 API adapter):
Item component compatibility was adapted from Horizon PR #8659,
commit ca17074fd90d9131f32d5c288c595a28eb94aa62 (MIT).
https://github.com/horizon-rust/horizon/commit/ca17074fd90d9131f32d5c288c595a28eb94aa62
Read/adapted: horizon-protocol item.rs, adapter/ids.rs and the dropped-component
constants in adapter/v*_ids.rs, including the upstream item byte fixtures.
Changes: native support modules and boundary hooks preserve the fix on the
official Nightly without replacing the base source revision or changing Ids.

MIT License

Copyright (c) 2026 Ludovic Marie and the Horizon contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
