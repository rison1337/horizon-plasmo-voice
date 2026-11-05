//! Launcher post-build step. Installs only this OS's checked, bundled runtime.
use horizon_plasmo_engine::{Result, denoise::Denoiser, hrtf::Hrtf, libraries, opus::Opus};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    env, fs,
    path::{Component, Path, PathBuf},
};

#[derive(Deserialize)]
struct Manifest {
    targets: HashMap<String, Vec<File>>,
    common: Vec<File>,
}
#[derive(Deserialize)]
struct File {
    source: String,
    destination: String,
    sha256: String,
}
fn relative(s: &str) -> Result<&Path> {
    let p = Path::new(s);
    if p.components().all(|c| matches!(c, Component::Normal(_))) && !s.contains('\\') {
        Ok(p)
    } else {
        Err(format!("Invalid voice runtime path: {s}"))
    }
}
fn checked_copy(from: &Path, to: &Path, file: &File) -> Result<()> {
    let source = from.join(relative(&file.source)?);
    let destination = to.join(relative(&file.destination)?);
    let bytes = fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != file.sha256 {
        return Err(format!("Voice runtime checksum mismatch: {}", file.source));
    }
    fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&destination, &bytes).map_err(|e| format!("{}: {e}", destination.display()))
}
fn verify() -> Result<()> {
    // No physical audio devices: exercise the installed codec and DSP APIs.
    let opus = Opus::load()?;
    let mut encoder = opus.encoder(48000)?;
    let mut decoder = opus.decoder(false)?;
    let packet = encoder.encode(&[0; 960], 1275)?;
    if decoder.decode(Some(&packet))?.len() != 960 {
        return Err("Installed Opus failed its round trip".into());
    }
    Denoiser::load()?.process(&mut [0; 480])?;
    let mut hrtf = Hrtf::new(3)?;
    hrtf.push(1, &[0; 960], [0., 0., -1.], 1.)?;
    if hrtf.render().len() != 1920 {
        return Err("Installed HRTF failed its loopback check".into());
    }
    Ok(())
}
fn run() -> Result<()> {
    if env::args().any(|arg| arg == "--check") {
        verify()?;
        println!(
            "Plasmo Voice: bundled Opus, RNNoise and HRTF work without library path overrides."
        );
        return Ok(());
    }
    let from = PathBuf::from(
        env::var_os("HORIZON_NATIVE_BUNDLE_DATA_DIR").ok_or("Missing bundle data directory")?,
    )
    .join("runtime");
    let to = PathBuf::from(
        env::var_os("HORIZON_NATIVE_DATA_DIR").ok_or("Missing installed data directory")?,
    );
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(from.join("manifest.json")).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let target = format!("{}-{}", env::consts::OS, env::consts::ARCH);
    let files = manifest.targets.get(&target).ok_or_else(|| {
        format!(
            "Plasmo Voice runtime does not support {target}; supported: Linux and Windows x86_64"
        )
    })?;
    for file in files.iter().chain(&manifest.common) {
        checked_copy(&from, &to, file)?;
    }
    libraries::set_directory(to.join("lib"))?;
    verify()?;
    fs::write(
        to.join("runtime-installed.json"),
        serde_json::json!({
            "schema": 1, "target": target, "files": files.len() + manifest.common.len(),
            "opus": true, "rnnoise": true, "hrtf_loopback": true,
        })
        .to_string(),
    )
    .map_err(|e| e.to_string())?;
    println!("Plasmo Voice: installed and verified Opus, RNNoise and HRTF for {target}.");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Plasmo Voice runtime: {e}");
        std::process::exit(1);
    }
}
