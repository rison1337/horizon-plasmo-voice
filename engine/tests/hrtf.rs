use horizon_plasmo_engine::hrtf::Hrtf;
#[test]
#[ignore = "requires the optional OpenAL Soft system library and HRTF data"]
fn loopback_hrtf_tracks_left_and_right_without_opening_audio_devices() {
    fn energy(x: f32, gain: f32) -> [f32; 2] {
        let mut renderer = Hrtf::new(5).expect("OpenAL Soft with HRTF required for explicit check");
        let mut energy = [0.; 2];
        for n in 0..8 {
            let pcm: Vec<_> = (0..960)
                .map(|i| (((n * 960 + i) as f32 * 0.0576).sin() * 500.) as i16)
                .collect();
            renderer.push(1, &pcm, [x, 0., -1.], gain).unwrap();
            let audio = renderer.render();
            for frame in audio.as_chunks::<2>().0 {
                assert!(frame.iter().all(|v| v.is_finite()));
                energy[0] += frame[0] * frame[0];
                energy[1] += frame[1] * frame[1];
            }
        }
        energy
    }
    let left = energy(-3., 1.);
    let right = energy(3., 1.);
    assert!(left[0] > left[1] * 1.1, "{left:?}");
    assert!(right[1] > right[0] * 1.1, "{right:?}");
    let normal = energy(0., 1.);
    let boosted = energy(0., 4.);
    let ratio = (boosted[0] + boosted[1]) / (normal[0] + normal[1]);
    assert!(
        (ratio - 16.).abs() < 1.,
        "HRTF must preserve slider boost: {ratio}"
    );
}
