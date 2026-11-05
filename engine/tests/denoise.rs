use horizon_plasmo_engine::{activation::level, denoise::Denoiser};
#[test]
#[ignore = "requires the optional RNNoise system library"]
fn rnnoise_reduces_steady_background_noise_without_recording_a_device() {
    let mut denoiser = Denoiser::load().expect("RNNoise must be installed for this explicit check");
    let mut seed = 7u32;
    let mut before = 0.;
    let mut after = 0.;
    for index in 0..100 {
        let mut pcm: Vec<i16> = (0..960)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                ((seed >> 16) as i16) / 16
            })
            .collect();
        let input = level(&pcm);
        denoiser.process(&mut pcm).unwrap();
        if index > 20 {
            before += input;
            after += level(&pcm);
        }
    }
    assert!(after < before * 0.7, "noise level {before} -> {after}");
}
