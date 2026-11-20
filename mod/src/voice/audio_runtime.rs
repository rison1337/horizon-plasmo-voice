//! Audio dependencies prepared offline from verified upstream source, never
//! downloaded or supplied as executable binaries in the native mod package.

pub struct File {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

pub use crate::runtime_cache::{
    built_directory, configure, failure_detail, prepare, request_prepare, retry, unavailable_reason,
};

/// Cancel this process's background compiler when the client is stopping.
pub fn shutdown() {
    crate::runtime_builder::stop();
}

pub const LICENSES: &[File] = &[
    File {
        name: "Opus-BSD.txt",
        bytes: include_bytes!("../../assets/plasmo_voice/audio/licenses/opus-bsd.txt"),
    },
    File {
        name: "RNNoise-BSD.txt",
        bytes: include_bytes!("../../assets/plasmo_voice/audio/licenses/rnnoise-bsd.txt"),
    },
    File {
        name: "OpenAL-Soft-LGPL.txt",
        bytes: include_bytes!("../../assets/plasmo_voice/audio/licenses/openal-soft-lgpl.txt"),
    },
];

pub const SOURCE_MANIFEST: &str = include_str!("../../assets/plasmo_voice/audio/sources.json");
pub const SOURCE_ARCHIVES: &[File] = &[
    File {
        name: "opus-1.6.1.tar.gz",
        bytes: include_bytes!("../../assets/plasmo_voice/audio/sources/opus-1.6.1.tar.gz"),
    },
    File {
        name: "rnnoise-0.2.tar.gz",
        bytes: include_bytes!("../../assets/plasmo_voice/audio/sources/rnnoise-0.2.tar.gz"),
    },
    File {
        name: "openal-soft-1.25.2.tar.gz",
        bytes: include_bytes!("../../assets/plasmo_voice/audio/sources/openal-soft-1.25.2.tar.gz"),
    },
];
