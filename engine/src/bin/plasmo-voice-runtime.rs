//! Developer audio-cache and codec/DSP checks; not an installer.
use horizon_plasmo_engine::{Result, audio_runtime, denoise::Denoiser, hrtf::Hrtf, opus::Opus};
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
    let args: Vec<_> = std::env::args().collect();
    let result = audio_runtime::prepare();
    if let Some(at) = args.iter().position(|a| a == "--expect-disabled") {
        let code = audio_runtime::unavailable_reason();
        if result.is_ok() || args.get(at + 1).map(String::as_str) != code {
            return Err(format!("unexpected audio preparation result: {code:?}"));
        }
        println!("Plasmo Voice: unavailable as expected ({})", code.unwrap());
        return Ok(());
    }
    result?;
    verify()?;
    println!("Plasmo Voice: verified-source Opus, RNNoise and HRTF pass without physical devices.");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
