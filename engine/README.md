# Voice engine and source-built audio runtime

This crate compiles unmodified upstream C/C++ dependencies during Cargo's build.
The native mod contains sources, never prebuilt audio libraries. It makes no
network requests. Exact original URLs, versions, licenses and SHA-256 values are
in [data/sources.json](data/sources.json). Compressed archives are upstream files,
not generated replacements; all are verified before extraction.

- Opus 1.6.1: BSD-3-Clause; generic scalar build.
- RNNoise 0.2: BSD-3-Clause; documented generic C source list.
- OpenAL Soft 1.25.2: LGPL-2.0-or-later; shared loopback/HRTF build with embedded
  upstream HRTF data. CPAL owns physical audio devices. OpenAL's implementation
  is not incorporated into the independent Rust code.

`build.rs` extracts into OUT_DIR, invokes the original CMake build definitions
through `data/CMakeLists.txt`, and exposes the three resulting shared libraries
to the post-build installer. Original archives/licenses are retained there too.
Only the original build integration in this crate is MIT; upstream source retains
its own notices and licenses. No upstream implementation was modified.

Supported targets: x86_64-unknown-linux-gnu and x86_64-pc-windows-gnu. Install C/C++
and CMake; on Windows GNU also install Ninja. Windows compiler support code is
linked into the shared libraries so no extra MinGW runtime DLLs are required.
The shared-library boundary remains replaceable for LGPL compliance.

From the repository root:

```sh
cargo test --workspace --locked
cargo test --workspace --test hrtf --test denoise -- --ignored
cargo run -p horizon-plasmo-engine --bin plasmo-voice-runtime -- --check
python3 tools/check_source_bundle.py dist/plasmo-voice-0.5.4.hnmod
```

The checks use synthetic audio and OpenAL loopback, without opening physical
microphones or speakers. Runtime installation requires the launcher's
HORIZON_NATIVE_DATA_DIR; it writes libraries, notices, complete corresponding
source and a hash receipt, then verifies the installed APIs.

The bundle declares this single crate. Its registry/build dependencies must
already exist in Horizon’s Cargo.lock. No nested Cargo package, inter-crate
path dependency, cmake crate or tar crate is included. The original build code
uses cc for compiler selection and sha2 for integrity, then invokes local CMake.
Missing tools, failed compilation and unsupported targets generate a disabled
build result instead of panicking. A disabled client refuses codec loading and
voice worker startup even if old installed libraries are present.
