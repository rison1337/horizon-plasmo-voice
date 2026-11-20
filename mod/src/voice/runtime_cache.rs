//! Offline preparation of verified audio sources. No Cargo build script,
//! downloaded executable, or compiler work on the game's render thread.
use crate::audio_runtime::{LICENSES, SOURCE_ARCHIVES, SOURCE_MANIFEST};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{Condvar, Mutex, OnceLock},
};

enum Phase {
    Cold,
    Building,
    Ready(PathBuf),
    Failed(&'static str, String),
}
struct Runtime {
    phase: Mutex<Phase>,
    changed: Condvar,
    cache: OnceLock<PathBuf>,
}
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime {
        phase: Mutex::new(Phase::Cold),
        changed: Condvar::new(),
        cache: OnceLock::new(),
    })
}
pub fn configure(directory: PathBuf) -> crate::Result<()> {
    if runtime().cache.get().is_some_and(|p| p == &directory) {
        return Ok(());
    }
    runtime()
        .cache
        .set(directory)
        .map_err(|_| "voice audio cache is already selected".into())
}
fn target() -> &'static str {
    if cfg!(all(
        target_arch = "x86_64",
        target_os = "linux",
        target_env = "gnu"
    )) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(
        target_arch = "x86_64",
        target_os = "windows",
        target_env = "gnu"
    )) {
        "x86_64-pc-windows-gnu"
    } else {
        "unsupported"
    }
}
fn cache() -> PathBuf {
    let base = runtime().cache.get_or_init(|| {
        std::env::var_os("PLASMO_AUDIO_CACHE")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("horizon-plasmo-audio"))
    });
    let mut hash = Sha256::new();
    hash.update(SOURCE_MANIFEST);
    hash.update(include_bytes!(
        "../../assets/plasmo_voice/audio/cmakelists.txt"
    ));
    hash.update(b"audio-cache-v1:O2,x86-64,generic,no-intrinsics");
    hash.update(target());
    base.join(format!("{}-{:x}", target(), hash.finalize()))
}
pub fn unavailable_reason() -> Option<&'static str> {
    match &*runtime().phase.lock().unwrap_or_else(|e| e.into_inner()) {
        Phase::Cold | Phase::Building => Some("preparing"),
        Phase::Ready(_) => None,
        Phase::Failed(code, _) => Some(code),
    }
}
pub fn failure_detail() -> Option<String> {
    match &*runtime().phase.lock().unwrap_or_else(|e| e.into_inner()) {
        Phase::Failed(_, detail) => Some(detail.clone()),
        _ => None,
    }
}
pub fn built_directory() -> PathBuf {
    match &*runtime().phase.lock().unwrap_or_else(|e| e.into_inner()) {
        Phase::Ready(dir) => dir.clone(),
        _ => cache().join("build/runtime"),
    }
}
/// Start once, without blocking UI input or rendering. Failed preparations do
/// not loop: the explicit reconnect/retry action starts another attempt.
pub fn request_prepare() {
    let mut phase = runtime().phase.lock().unwrap_or_else(|e| e.into_inner());
    if !matches!(*phase, Phase::Cold) {
        return;
    }
    *phase = Phase::Building;
    drop(phase);
    if let Err(error) = std::thread::Builder::new()
        .name("Plasmo audio preparation".into())
        .spawn(finish)
    {
        *runtime().phase.lock().unwrap_or_else(|e| e.into_inner()) =
            Phase::Failed("build_failed", error.to_string());
        runtime().changed.notify_all();
    }
}
pub fn retry() {
    let mut phase = runtime().phase.lock().unwrap_or_else(|e| e.into_inner());
    if matches!(*phase, Phase::Failed(_, _)) {
        *phase = Phase::Cold;
    }
    drop(phase);
    request_prepare();
}
/// For audio workers and developer checks. UI hooks only request preparation
/// and read the state; they never wait for a compiler.
pub fn prepare() -> crate::Result<()> {
    let mut phase = runtime().phase.lock().unwrap_or_else(|e| e.into_inner());
    loop {
        match &*phase {
            Phase::Ready(_) => return Ok(()),
            Phase::Failed(code, detail) => {
                return Err(format!("Plasmo Voice disabled ({code}): {detail}"));
            }
            Phase::Building => {
                phase = runtime()
                    .changed
                    .wait(phase)
                    .unwrap_or_else(|e| e.into_inner());
            }
            Phase::Cold => {
                *phase = Phase::Building;
                drop(phase);
                finish();
                phase = runtime().phase.lock().unwrap_or_else(|e| e.into_inner());
            }
        }
    }
}
fn finish() {
    let result = prepare_cached();
    *runtime().phase.lock().unwrap_or_else(|e| e.into_inner()) = match result {
        Ok(dir) => Phase::Ready(dir),
        Err(crate::runtime_builder::Unavailable(code, detail)) => Phase::Failed(code, detail),
    };
    runtime().changed.notify_all();
}
fn prepare_cached() -> Result<PathBuf, crate::runtime_builder::Unavailable> {
    use crate::runtime_builder::Unavailable;
    let io = |e: std::io::Error| Unavailable("build_failed", e.to_string());
    if target() == "unsupported" {
        return Err(Unavailable(
            "unsupported_target",
            "audio currently supports Linux/Windows GNU x86_64".into(),
        ));
    }
    let root = cache();
    fs::create_dir_all(&root).map_err(io)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("prepare.lock"))
        .map_err(io)?;
    lock.lock().map_err(io)?;
    let out = root.join("build");
    let library_dir = out.join("runtime");
    let names: &[&str] = if cfg!(windows) {
        &["libopus-0.dll", "librnnoise-0.dll", "soft_oal.dll"]
    } else {
        &["libopus.so.0", "librnnoise.so.0", "libopenal.so.1"]
    };
    let valid = fs::read(root.join("receipt.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<BTreeMap<String, String>>(&b).ok())
        .is_some_and(|receipt| {
            receipt.len() == names.len()
                && names.iter().all(|name| {
                    fs::read(library_dir.join(name)).is_ok_and(|bytes| {
                        receipt.get(*name) == Some(&format!("{:x}", Sha256::digest(bytes)))
                    })
                })
        });
    if valid {
        return Ok(library_dir);
    }
    let input = root.join("input/data");
    fs::create_dir_all(input.join("sources")).map_err(io)?;
    fs::create_dir_all(&out).map_err(io)?;
    fs::write(
        input.join("CMakeLists.txt"),
        include_bytes!("../../assets/plasmo_voice/audio/cmakelists.txt"),
    )
    .map_err(io)?;
    for file in SOURCE_ARCHIVES {
        fs::write(input.join("sources").join(file.name), file.bytes).map_err(io)?;
    }
    for file in LICENSES {
        fs::write(root.join(file.name), file.bytes).map_err(io)?;
    }
    fs::write(root.join("sources.json"), SOURCE_MANIFEST).map_err(io)?;
    fs::write(out.join("audio-build.log"), []).map_err(io)?;
    let actual = crate::runtime_builder::build(&root.join("input"), &out, target())?;
    if actual.len() != names.len() {
        return Err(Unavailable(
            "build_failed",
            "audio output inventory differs".into(),
        ));
    }
    let mut receipt = BTreeMap::new();
    for name in names {
        let bytes = fs::read(library_dir.join(name)).map_err(io)?;
        receipt.insert(*name, format!("{:x}", Sha256::digest(bytes)));
    }
    fs::write(
        root.join("receipt.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .map_err(io)?;
    Ok(library_dir)
}
