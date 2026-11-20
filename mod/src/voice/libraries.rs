//! Load only the verified audio cache, using absolute paths.
use crate::Result;
use libloading::Library;

/// The caller must use the loaded symbols with the library's published ABI.
/// A present bundled library that cannot load is reported, never silently replaced.
pub(crate) unsafe fn load(names: &[&str]) -> Result<Library> {
    crate::audio_runtime::prepare()?;
    // Only this verified cache is eligible. Older native-data installations,
    // arbitrary system codecs and DLLs from PATH never override its output.
    for name in names {
        let path = crate::audio_runtime::built_directory().join(name);
        if path.is_file() {
            return unsafe { Library::new(&path) }.map_err(|e| format!("{}: {e}", path.display()));
        }
    }
    Err(format!("Voice library missing: {}", names.join(" / ")))
}
