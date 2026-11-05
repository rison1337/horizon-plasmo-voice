//! Resolve the runtime shipped by the native mod relative to the game executable.
//! Absolute paths avoid dependence on the launcher's working directory or PATH.
use crate::Result;
use libloading::Library;
use std::{path::PathBuf, sync::OnceLock};

static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

/// Used by the launcher's post-build program before any audio API is opened.
pub fn set_directory(path: PathBuf) -> Result<()> {
    DIRECTORY
        .set(path)
        .map_err(|_| "voice runtime directory already selected".into())
}

pub fn directory() -> Option<PathBuf> {
    DIRECTORY.get().cloned().or_else(|| {
        Some(
            std::env::current_exe()
                .ok()?
                .parent()?
                .join("native-data/plasmo_voice/lib"),
        )
    })
}

/// The caller must use the loaded symbols with the library's published ABI.
/// A present bundled library that cannot load is reported, never silently replaced.
pub(crate) unsafe fn load(names: &[&str]) -> Result<Library> {
    if let Some(dir) = directory() {
        for name in names {
            let path = dir.join(name);
            if path.is_file() {
                // SAFETY: shipped library, retained by the caller while symbols live.
                return unsafe { Library::new(&path) }
                    .map_err(|e| format!("{}: {e}", path.display()));
            }
        }
    }
    // Developers and existing installations may still use system libraries.
    for name in names {
        // SAFETY: callers supply only the fixed public API names below.
        if let Ok(lib) = unsafe { Library::new(name) } {
            return Ok(lib);
        }
    }
    Err(format!("Voice library missing: {}", names.join(" / ")))
}
