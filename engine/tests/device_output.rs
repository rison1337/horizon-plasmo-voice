//! Explicit null-driver checks never open a physical speaker or microphone.
use horizon_plasmo_engine::audio::{Output, devices};

#[test]
#[ignore = "run with ALSOFT_DRIVERS=null; checks playback and bounded underrun recovery"]
fn null_output_queues_audio_recovers_after_underrun_and_closes_cleanly() {
    assert_eq!(std::env::var("ALSOFT_DRIVERS").as_deref(), Ok("null"));
    let output_names = devices(false);
    assert!(!output_names.is_empty());
    let mut output = Output::buffered(&output_names[0], 3).unwrap();
    for frame in 0..12 {
        let data: Vec<f32> = (0..1920)
            .map(|i| ((frame * 960 + i / 2) as f32 * 0.02).sin() * 0.02)
            .collect();
        output.write(&data);
        assert!(!output.failed());
        std::thread::sleep(std::time::Duration::from_millis(if frame == 4 {
            110
        } else {
            20
        }));
    }
    drop(output);
    let mut reopened = Output::open("").unwrap();
    reopened.write(&[0.0; 1920]);
    assert!(!reopened.failed());
}
