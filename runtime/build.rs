use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::{env, fs, io::Read, path::Path};

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

fn extract(root: &Path, output: &Path, name: &str, checksum: &str) {
    let archive = root.join(format!("data/sources/{name}.tar.gz"));
    println!("cargo:rerun-if-changed={}", archive.display());
    let bytes = fs::read(&archive).expect("read vendored source archive");
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        checksum,
        "source archive checksum: {name}"
    );
    let marker = output.join(format!("{name}.sha256"));
    if fs::read_to_string(&marker).is_ok_and(|value| value == checksum) {
        return;
    }
    let source = output.join(name);
    if source.exists() {
        fs::remove_dir_all(&source).expect("remove previous extracted source");
    }
    let mut expanded = 0u64;
    for entry in tar::Archive::new(GzDecoder::new(&bytes[..]))
        .entries()
        .expect("read source tar")
    {
        let mut entry = entry.expect("read source entry");
        let kind = entry.header().entry_type();
        if kind.is_pax_global_extensions() {
            continue;
        }
        let path = entry.path().expect("source path").into_owned();
        assert!(
            path.starts_with(name),
            "unexpected source root: {} in {name}",
            path.display()
        );
        assert!(
            kind.is_file() || kind.is_dir(),
            "source archives must contain only files/directories"
        );
        expanded += entry.size();
        assert!(
            expanded <= 256 * 1024 * 1024,
            "source archive expansion limit"
        );
        assert!(
            entry.unpack_in(output).expect("extract source entry"),
            "unsafe source path"
        );
    }
    fs::write(marker, checksum).expect("write verified source marker");
}

fn copy_library(from: &Path, to: &Path, names: &[&str], canonical: &str) {
    let file = names
        .iter()
        .map(|name| from.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| {
            panic!(
                "source build did not produce {canonical} in {}",
                from.display()
            )
        });
    // An ordinary file in OUT_DIR, never a prebuilt file from the bundle.
    fs::copy(file, to.join(canonical)).expect("copy built shared library");
}

fn main() {
    let root = env::current_dir().expect("crate directory");
    let out = std::path::PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let sources = out.join("sources");
    fs::create_dir_all(&sources).expect("source directory");
    for (name, sha) in SOURCES {
        extract(&root, &sources, name, sha);
    }
    let target = env::var("TARGET").expect("TARGET");
    assert!(
        target == "x86_64-unknown-linux-gnu" || target == "x86_64-pc-windows-gnu",
        "source runtime currently supports Linux GNU / Windows GNU x86_64"
    );
    println!("cargo:rerun-if-changed=data/CMakeLists.txt");
    let mut config = cmake::Config::new(root.join("data"));
    config
        .out_dir(out.join("cmake"))
        .profile("Release")
        .define("PLASMO_SOURCES", &sources)
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        .define("CMAKE_INSTALL_LIBDIR", "lib")
        .define("CMAKE_C_FLAGS", "-O2 -march=x86-64 -mtune=generic")
        .define("CMAKE_CXX_FLAGS", "-O2 -march=x86-64 -mtune=generic");
    if target.ends_with("windows-gnu") {
        config.generator("Ninja");
        config.define(
            "CMAKE_SHARED_LINKER_FLAGS",
            "-static -static-libgcc -static-libstdc++",
        );
    }
    let install = config.build();
    let runtime = out.join("runtime");
    fs::create_dir_all(&runtime).expect("runtime directory");
    if target.contains("windows") {
        copy_library(
            &install.join("bin"),
            &runtime,
            &["libopus-0.dll", "opus.dll", "libopus.dll"],
            "libopus-0.dll",
        );
        copy_library(
            &install.join("bin"),
            &runtime,
            &["librnnoise.dll", "rnnoise.dll"],
            "librnnoise-0.dll",
        );
        copy_library(
            &install.join("bin"),
            &runtime,
            &["soft_oal.dll", "OpenAL32.dll", "libopenal.dll"],
            "soft_oal.dll",
        );
    } else {
        copy_library(
            &install.join("lib"),
            &runtime,
            &["libopus.so.0", "libopus.so"],
            "libopus.so.0",
        );
        copy_library(
            &install.join("lib"),
            &runtime,
            &["librnnoise.so.0", "librnnoise.so"],
            "librnnoise.so.0",
        );
        copy_library(
            &install.join("lib"),
            &runtime,
            &["libopenal.so.1", "libopenal.so"],
            "libopenal.so.1",
        );
    }
    // Only local compiler output is embedded by the post-build installer.
    println!("cargo:rustc-env=PLASMO_BUILT_RUNTIME={}", runtime.display());
    println!("cargo:rustc-env=PLASMO_BUILT_SOURCES={}", sources.display());
    // Read enough to ensure installation never silently ships a missing output.
    for file in fs::read_dir(&runtime).expect("built outputs") {
        let mut magic = [0; 4];
        fs::File::open(file.unwrap().path())
            .unwrap()
            .read_exact(&mut magic)
            .unwrap();
        assert!(
            magic == *b"\x7fELF" || magic[..2] == *b"MZ",
            "invalid shared library output"
        );
    }
}
