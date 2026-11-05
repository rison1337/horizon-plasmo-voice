Plasmo Voice bundled runtime (x86_64)

Opus 1.6.1 and RNNoise 0.2: BSD. Their license notices are in licenses/.
Linux OpenAL Soft: unmodified LWJGL 3.3.3 native binary (GLIBC >= 2.17),
source https://github.com/LWJGL-CI/OpenAL-Soft/commit/d9fed51aa6391debc31dbbca550f055c980afe70
Windows OpenAL Soft: unmodified official 1.25.2 x64 binary.
These libraries are dynamically loaded. OpenAL Soft is LGPL; complete matching
source archives are in sources/. You may replace the libraries in lib/.
The native mod implementation is MIT and contains no OpenAL implementation.
Original Plasmo Voice implementation and game resources are not included.

Linux Opus/RNNoise rebuilt for baseline x86_64 with runtime CPU detection:
CFLAGS=-O2 -march=x86-64 -mtune=generic; no assumed AVX/SSE4 requirement.
Opus runtime detection is enabled by default; RNNoise uses --enable-x86-rtcd.
Linux Opus links the GLIBC 2.29 math ABI; newer host-only symbol versions and
packed-relative-relocation requirements are excluded. Linux requires GLIBC 2.29+.
Opus source SHA256: 6ffcb593207be92584df15b32466ed64bbec99109f007c82205f0194572411a1
RNNoise source SHA256: 90fce4b00b9ff24c08dbfe31b82ffd43bae383d85c5535676d28b0a2b11c0d37
The upstream versions, Windows binaries and OpenAL payloads are unchanged.
