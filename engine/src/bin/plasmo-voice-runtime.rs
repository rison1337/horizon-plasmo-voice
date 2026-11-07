//! Launcher post-build step. Installs this Cargo build's source-built libraries.
use horizon_plasmo_engine::{Result, denoise::Denoiser, hrtf::Hrtf, libraries, opus::Opus};
use horizon_plasmo_runtime::{LIBRARIES, LICENSES, SOURCE_ARCHIVES, SOURCE_MANIFEST};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn verify() -> Result<()> {
    // No physical audio devices: exercise the built codec and DSP APIs.
    let opus = Opus::load()?;
    let mut encoder = opus.encoder(48000)?;
    let mut decoder = opus.decoder(false)?;
    let packet = encoder.encode(&[0; 960], 1275)?;
    if decoder.decode(Some(&packet))?.len() != 960 {
        return Err("Source-built Opus failed its round trip".into());
    }
    Denoiser::load()?.process(&mut [0; 480])?;
    let mut hrtf = Hrtf::new(3)?;
    hrtf.push(1, &[0; 960], [0., 0., -1.], 1.)?;
    if hrtf.render().len() != 1920 {
        return Err("Source-built HRTF failed its loopback check".into());
    }
    Ok(())
}
fn run() -> Result<()> {
    if env::args().any(|arg| arg == "--check") {
        // Deliberately bypass older installed binaries for a source-build test.
        libraries::set_directory(horizon_plasmo_runtime::built_directory().into())?;
        verify()?;
        println!(
            "Plasmo Voice: source-built Opus, RNNoise and HRTF pass without physical devices."
        );
        return Ok(());
    }
    let to = PathBuf::from(
        env::var_os("HORIZON_NATIVE_DATA_DIR").ok_or("Missing installed data directory")?,
    );
    let mut installed = Vec::new();
    for (folder, files) in [
        ("lib", LIBRARIES),
        ("licenses", LICENSES),
        ("sources", SOURCE_ARCHIVES),
    ] {
        let destination = to.join(folder);
        fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
        for file in files {
            let path = destination.join(file.name);
            // Do not rewrite an identical loaded DLL during a repeated build.
            if !fs::read(&path).is_ok_and(|bytes| bytes == file.bytes) {
                fs::write(&path, file.bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            }
            installed.push(serde_json::json!({"path":format!("{folder}/{}",file.name),"sha256":format!("{:x}",Sha256::digest(file.bytes))}));
        }
    }
    fs::write(to.join("sources.json"), SOURCE_MANIFEST).map_err(|e| e.to_string())?;
    libraries::set_directory(to.join("lib"))?;
    verify()?;
    fs::write(
        to.join("runtime-installed.json"),
        serde_json::json!({
            "schema":2,"target":format!("{}-{}",env::consts::OS,env::consts::ARCH),
            "built_from_source":true,"files":installed,
            "opus":true,"rnnoise":true,"hrtf_loopback":true,
        })
        .to_string(),
    )
    .map_err(|e| e.to_string())?;
    println!("Plasmo Voice: installed and verified source-built Opus, RNNoise and HRTF.");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Plasmo Voice runtime: {e}");
        std::process::exit(1);
    }
}
