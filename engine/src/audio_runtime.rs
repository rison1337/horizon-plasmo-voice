//! Audio dependencies built from verified upstream source by Cargo, never
//! downloaded or supplied as executable binaries in the native mod package.
use std::path::Path;

pub struct File {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/runtime-files.rs"));

/// None means this build's native codec/DSP runtime is available.
pub fn unavailable_reason() -> Option<&'static str> {
    (!DISABLED_CODE.is_empty()).then_some(DISABLED_CODE)
}

pub const LICENSES: &[File] = &[
    File {
        name: "Opus-BSD.txt",
        bytes: include_bytes!("../data/licenses/Opus-BSD.txt"),
    },
    File {
        name: "RNNoise-BSD.txt",
        bytes: include_bytes!("../data/licenses/RNNoise-BSD.txt"),
    },
    File {
        name: "OpenAL-Soft-LGPL.txt",
        bytes: include_bytes!("../data/licenses/OpenAL-Soft-LGPL.txt"),
    },
];

/// Developer/test fallback. Release installations use the post-build output
/// next to the game executable; this directory belongs to this Cargo build.
pub fn built_directory() -> &'static Path {
    Path::new(BUILT_DIRECTORY)
}

pub const SOURCE_MANIFEST: &str = include_str!("../data/sources.json");
pub const SOURCE_ARCHIVES: &[File] = &[
    File {
        name: "opus-1.6.1.tar.gz",
        bytes: include_bytes!("../data/sources/opus-1.6.1.tar.gz"),
    },
    File {
        name: "rnnoise-0.2.tar.gz",
        bytes: include_bytes!("../data/sources/rnnoise-0.2.tar.gz"),
    },
    File {
        name: "openal-soft-1.25.2.tar.gz",
        bytes: include_bytes!("../data/sources/openal-soft-1.25.2.tar.gz"),
    },
];
