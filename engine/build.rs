//! Build only verified upstream sources; an unavailable native toolchain disables
//! voice instead of aborting the player's combined native-mod build.
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

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

struct Unavailable(&'static str, String);
type Result<T> = std::result::Result<T, Unavailable>;
fn io(error: std::io::Error) -> Unavailable {
    Unavailable("build_failed", error.to_string())
}
fn run(command: &mut Command, out: &Path) -> Result<()> {
    let output = command.output().map_err(io)?;
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join("audio-build.log"))
        .map_err(io)?;
    use std::io::Write;
    writeln!(log, "{command:?}").map_err(io)?;
    log.write_all(&output.stdout).map_err(io)?;
    log.write_all(&output.stderr).map_err(io)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(Unavailable(
            "build_failed",
            format!(
                "{command:?}: {}; see {}",
                output.status,
                out.join("audio-build.log").display()
            ),
        ))
    }
}
fn tool_available(tool: &std::ffi::OsStr) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}
fn build(root: &Path, out: &Path, target: &str) -> Result<Vec<&'static str>> {
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
    let compiler = |cpp| {
        cc::Build::new()
            .cpp(cpp)
            .try_get_compiler()
            .map_err(|e| Unavailable("missing_compiler", e.to_string()))
    };
    let c = compiler(false)?;
    let cxx = compiler(true)?;
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
        .arg(format!("-DCMAKE_C_COMPILER={}", c.path().display()))
        .arg(format!("-DCMAKE_CXX_COMPILER={}", cxx.path().display()))
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
        if !env::var("HOST").unwrap_or_default().contains("windows") {
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
            .arg(env::var("NUM_JOBS").unwrap_or_else(|_| "2".into()))
            .args(["--target", "install"]),
        out,
    )?;
    let outputs = if target.ends_with("windows-gnu") {
        vec![
            ("bin", "libopus-0.dll", "libopus-0.dll"),
            ("bin", "librnnoise.dll", "librnnoise-0.dll"),
            ("bin", "soft_oal.dll", "soft_oal.dll"),
        ]
    } else {
        vec![
            ("lib", "libopus.so.0", "libopus.so.0"),
            ("lib", "librnnoise.so.0", "librnnoise.so.0"),
            ("lib", "libopenal.so.1", "libopenal.so.1"),
        ]
    };
    let runtime = out.join("runtime");
    fs::create_dir_all(&runtime).map_err(io)?;
    let mut names = Vec::new();
    for (folder, source, name) in outputs {
        let bytes = fs::read(install.join(folder).join(source)).map_err(io)?;
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
fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output directory"));
    for path in ["build.rs", "data/CMakeLists.txt", "data/sources"] {
        println!("cargo:rerun-if-changed={path}");
    }
    for key in ["CMAKE", "PATH"] {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let target = env::var("TARGET").unwrap_or_default();
    let (names, code) = match build(&root, &out, &target) {
        Ok(names) => (names, ""),
        Err(Unavailable(code, detail)) => {
            if code != "unsupported_target" {
                // Keep a disabled toolchain build retryable. Installing CMake
                // into an existing PATH must recover on Rebuild without clean.
                println!(
                    "cargo:rerun-if-changed={}",
                    out.join("retry-audio-build").display()
                );
            }
            println!(
                "cargo:warning=Plasmo Voice disabled: {}",
                detail.replace(['\r', '\n'], " ")
            );
            (Vec::new(), code)
        }
    };
    let directory = out.join("runtime");
    let mut generated = format!(
        "pub const DISABLED_CODE: &str = {code:?};\npub const BUILT_DIRECTORY: &str = {:?};\npub const LIBRARIES: &[File] = &[\n",
        directory.to_string_lossy()
    );
    for name in names {
        generated.push_str(&format!(
            "File {{ name: {name:?}, bytes: include_bytes!({:?}) }},\n",
            directory.join(name).to_string_lossy()
        ));
    }
    generated.push_str("];\n");
    fs::write(out.join("runtime-files.rs"), generated)
        .expect("write build result into Cargo OUT_DIR");
}
