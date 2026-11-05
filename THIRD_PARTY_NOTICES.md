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
The native .hnmod includes dynamically loaded Opus 1.6.1 and RNNoise 0.2 (BSD),
and unmodified OpenAL Soft binaries (LGPL): Linux from LWJGL 3.3.3 at
LWJGL-CI/OpenAL-Soft d9fed51aa6391debc31dbbca550f055c980afe70, Windows 1.25.2.
Their license notices and complete corresponding OpenAL source archives are
in data/runtime/licenses/ and data/runtime/sources/ and are installed beside
the libraries. The mod is MIT; no OpenAL implementation was read or copied into
its implementation. Libraries can be replaced in native-data/plasmo_voice/lib/.
UI icons are code-drawn native primitives; original mod graphic assets are not bundled.

The 0.5.1 Linux Opus/RNNoise binaries are unmodified upstream releases rebuilt
with baseline x86_64 flags and runtime CPU detection. Build/license references:
- Opus 1.6.1 (BSD), COPYING in the release archive:
  https://downloads.xiph.org/releases/opus/opus-1.6.1.tar.gz
- RNNoise 0.2 (BSD), COPYING in the release archive:
  https://github.com/xiph/rnnoise/releases/tag/v0.2
Only their license/build configuration and published APIs were consulted;
no foreign codec/DSP implementation was copied into the Rust engine.


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
