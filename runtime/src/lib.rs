//! Audio dependencies built from verified upstream source by Cargo, never
//! downloaded or supplied as executable binaries in the native mod package.
use std::path::Path;

pub struct File {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

#[cfg(target_os = "linux")]
pub const LIBRARIES: &[File] = &[
    File {
        name: "libopus.so.0",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_RUNTIME"), "/libopus.so.0")),
    },
    File {
        name: "librnnoise.so.0",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_RUNTIME"), "/librnnoise.so.0")),
    },
    File {
        name: "libopenal.so.1",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_RUNTIME"), "/libopenal.so.1")),
    },
];
#[cfg(target_os = "windows")]
pub const LIBRARIES: &[File] = &[
    File {
        name: "libopus-0.dll",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_RUNTIME"), "/libopus-0.dll")),
    },
    File {
        name: "librnnoise-0.dll",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_RUNTIME"), "/librnnoise-0.dll")),
    },
    File {
        name: "soft_oal.dll",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_RUNTIME"), "/soft_oal.dll")),
    },
];

pub const LICENSES: &[File] = &[
    File {
        name: "Opus-BSD.txt",
        bytes: include_bytes!(concat!(env!("PLASMO_BUILT_SOURCES"), "/opus-1.6.1/COPYING")),
    },
    File {
        name: "RNNoise-BSD.txt",
        bytes: include_bytes!(concat!(
            env!("PLASMO_BUILT_SOURCES"),
            "/rnnoise-0.2/COPYING"
        )),
    },
    File {
        name: "OpenAL-Soft-LGPL.txt",
        bytes: include_bytes!(concat!(
            env!("PLASMO_BUILT_SOURCES"),
            "/openal-soft-1.25.2/COPYING"
        )),
    },
];

/// Developer/test fallback. Release installations use the post-build output
/// next to the game executable; this directory belongs to this Cargo build.
pub fn built_directory() -> &'static Path {
    Path::new(env!("PLASMO_BUILT_RUNTIME"))
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
