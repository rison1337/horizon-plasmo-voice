//! The offline audio preparation worker. All compiler inputs are hash-verified
//! source archives, and every write stays inside the selected audio cache.
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::Path,
    process::{Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

static STOPPING: AtomicBool = AtomicBool::new(false);
static COMPILER: Mutex<Option<u32>> = Mutex::new(None);

pub(crate) fn stop() {
    STOPPING.store(true, Ordering::Release);
    if let Some(pid) = *COMPILER.lock().unwrap_or_else(|e| e.into_inner()) {
        terminate(pid);
    }
}
fn terminate(pid: u32) {
    #[cfg(unix)]
    {
        unsafe extern "C" {
            fn kill(pid: i32, signal: i32) -> i32;
        }
        // The child owns its process group; terminate its compiler descendants
        // too, so closing the game never leaves a CMake job using this cache.
        unsafe {
            kill(-(pid as i32), 15);
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

const SOURCES: [(&str, &str); 3] = [
    (
        "opus-1.6.1",
        "6ffcb593207be92584df15b32466ed64bbec99109f007c82205f0194572411a1",
    ),
    (
        "rnnoise-0.2",
        "90fce4b00b9ff24c08dbfe31b82ffd43bae383d85c5535676d28b0a2b11c0d37",
    ),
    (
        "openal-soft-1.25.2",
        "fb27e5839aa11f0e5b9d33756965291fad5d6909ab928ea1f796f4a1a6877894",
    ),
];

pub(crate) struct Unavailable(pub(crate) &'static str, pub(crate) String);
type Result<T> = std::result::Result<T, Unavailable>;
fn io(error: std::io::Error) -> Unavailable {
    Unavailable("build_failed", error.to_string())
}
fn run(command: &mut Command, out: &Path) -> Result<()> {
    if STOPPING.load(Ordering::Acquire) {
        return Err(Unavailable("build_failed", "game is stopping".into()));
    }
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join("audio-build.log"))
        .map_err(io)?;
    use std::io::Write;
    writeln!(log, "{command:?}").map_err(io)?;
    command
        .stdout(Stdio::from(log.try_clone().map_err(io)?))
        .stderr(Stdio::from(log));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut active = COMPILER.lock().unwrap_or_else(|e| e.into_inner());
    if STOPPING.load(Ordering::Acquire) {
        return Err(Unavailable("build_failed", "game is stopping".into()));
    }
    let mut child = command.spawn().map_err(io)?;
    *active = Some(child.id());
    drop(active);
    let status = loop {
        if STOPPING.load(Ordering::Acquire) {
            terminate(child.id());
        }
        if let Some(status) = child.try_wait().map_err(io)? {
            break status;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    *COMPILER.lock().unwrap_or_else(|e| e.into_inner()) = None;
    if status.success() {
        Ok(())
    } else {
        Err(Unavailable(
            "build_failed",
            format!(
                "{command:?}: {}; see {}",
                status,
                out.join("audio-build.log").display()
            ),
        ))
    }
}
fn tool_available(tool: &std::ffi::OsStr) -> bool {
    let mut command = Command::new(tool);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}
pub(crate) fn build(root: &Path, out: &Path, target: &str) -> Result<Vec<&'static str>> {
    if !matches!(target, "x86_64-unknown-linux-gnu" | "x86_64-pc-windows-gnu") {
        return Err(Unavailable(
            "unsupported_target",
            format!("unsupported target {target}"),
        ));
    }
    let cmake = env::var_os("CMAKE").unwrap_or_else(|| "cmake".into());
    if !tool_available(&cmake) {
        return Err(Unavailable(
            "missing_cmake",
            "CMake is missing or cannot run".into(),
        ));
    }
    if target.ends_with("windows-gnu") && !tool_available(std::ffi::OsStr::new("ninja")) {
        return Err(Unavailable(
            "missing_ninja",
            "Ninja is missing or cannot run".into(),
        ));
    }
    let c = env::var_os("CC").unwrap_or_else(|| if cfg!(windows) { "gcc" } else { "cc" }.into());
    let cxx =
        env::var_os("CXX").unwrap_or_else(|| if cfg!(windows) { "g++" } else { "c++" }.into());
    if !tool_available(&c) || !tool_available(&cxx) {
        return Err(Unavailable(
            "missing_compiler",
            "C/C++ compiler is missing or cannot run".into(),
        ));
    }
    let sources = out.join("sources");
    fs::create_dir_all(&sources).map_err(io)?;
    for (name, checksum) in SOURCES {
        let archive = root.join(format!("data/sources/{name}.tar.gz"));
        let bytes = fs::read(&archive).map_err(io)?;
        if format!("{:x}", Sha256::digest(bytes)) != checksum {
            return Err(Unavailable(
                "source_integrity",
                format!("source archive checksum mismatch: {name}"),
            ));
        }
        let marker = sources.join(format!("{name}.sha256"));
        if !fs::read_to_string(&marker).is_ok_and(|s| s == checksum) {
            let source = sources.join(name);
            if source.exists() {
                fs::remove_dir_all(source).map_err(io)?;
            }
            // These exact upstream archives contain only safe, regular members;
            // the package inspector checks every member. Verify bytes BEFORE
            // asking CMake to extract them, always under this build's OUT_DIR.
            run(
                Command::new(&cmake)
                    .args(["-E", "tar", "xzf"])
                    .arg(&archive)
                    .current_dir(&sources),
                out,
            )?;
            fs::write(marker, checksum).map_err(io)?;
        }
    }
    let build = out.join("cmake");
    let install = out.join("install");
    // The launcher shares Cargo output between staged source directories.
    // CMake caches its source path, so a different stage must configure afresh.
    let source_dir = root.join("data");
    let normalize = |s: &str| s.replace('\\', "/");
    if fs::read_to_string(build.join("CMakeCache.txt"))
        .ok()
        .and_then(|cache| {
            cache.lines().find_map(|line| {
                line.strip_prefix("CMAKE_HOME_DIRECTORY:INTERNAL=")
                    .map(str::to_owned)
            })
        })
        .is_some_and(|path| normalize(&path) != normalize(&source_dir.to_string_lossy()))
    {
        fs::remove_dir_all(&build).map_err(io)?;
    }
    let mut config = Command::new(&cmake);
    config
        .arg("-S")
        .arg(root.join("data"))
        .arg("-B")
        .arg(&build)
        .arg(format!("-DPLASMO_SOURCES={}", sources.display()))
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install.display()))
        .arg("-DCMAKE_INSTALL_LIBDIR=lib")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DCMAKE_POLICY_VERSION_MINIMUM=3.5")
        .arg(format!("-DCMAKE_C_COMPILER={}", Path::new(&c).display()))
        .arg(format!(
            "-DCMAKE_CXX_COMPILER={}",
            Path::new(&cxx).display()
        ))
        .arg("-DCMAKE_C_FLAGS=-O2 -march=x86-64 -mtune=generic")
        .arg("-DCMAKE_CXX_FLAGS=-O2 -march=x86-64 -mtune=generic");
    if target.ends_with("windows-gnu") {
        config.args([
            "-G",
            "Ninja",
            "-DCMAKE_SHARED_LINKER_FLAGS=-static -static-libgcc -static-libstdc++",
        ]);
        // A Windows Rust installation often has an MSVC host even when the
        // mod targets GNU. MSYS2 provides unprefixed windres.exe in that case;
        // the prefixed resource compiler is only for cross-OS builds.
        if !cfg!(windows) {
            config
                .arg("-DCMAKE_SYSTEM_NAME=Windows")
                .arg("-DCMAKE_RC_COMPILER=x86_64-w64-mingw32-windres");
        }
    } else {
        config.args(["-G", "Unix Makefiles"]);
    }
    run(&mut config, out)?;
    run(
        Command::new(&cmake)
            .arg("--build")
            .arg(&build)
            .args(["--config", "Release", "--parallel"])
            .arg("1")
            .args(["--target", "install"]),
        out,
    )?;
    let outputs: Vec<(&str, &[&str], &str)> = if target.ends_with("windows-gnu") {
        vec![
            (
                "bin",
                &["libopus-0.dll", "opus.dll", "libopus.dll"],
                "libopus-0.dll",
            ),
            (
                "bin",
                &["librnnoise.dll", "rnnoise.dll"],
                "librnnoise-0.dll",
            ),
            (
                "bin",
                &["soft_oal.dll", "OpenAL32.dll", "libopenal.dll"],
                "soft_oal.dll",
            ),
        ]
    } else {
        vec![
            ("lib", &["libopus.so.0", "libopus.so"], "libopus.so.0"),
            (
                "lib",
                &["librnnoise.so.0", "librnnoise.so"],
                "librnnoise.so.0",
            ),
            ("lib", &["libopenal.so.1", "libopenal.so"], "libopenal.so.1"),
        ]
    };
    let runtime = out.join("runtime");
    fs::create_dir_all(&runtime).map_err(io)?;
    let mut names = Vec::new();
    for (folder, candidates, name) in outputs {
        let directory = install.join(folder);
        let source = candidates
            .iter()
            .map(|candidate| directory.join(candidate))
            .find(|path| path.is_file())
            .ok_or_else(|| {
                Unavailable(
                    "build_failed",
                    format!(
                        "compiler did not produce {name}; checked {candidates:?} in {}",
                        directory.display()
                    ),
                )
            })?;
        let bytes = fs::read(source).map_err(io)?;
        if !(bytes.starts_with(b"\x7fELF") || bytes.starts_with(b"MZ")) {
            return Err(Unavailable(
                "build_failed",
                format!("invalid compiler output: {name}"),
            ));
        }
        fs::write(runtime.join(name), bytes).map_err(io)?;
        names.push(name);
    }
    Ok(names)
}
