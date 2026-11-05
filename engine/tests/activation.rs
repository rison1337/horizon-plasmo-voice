use horizon_plasmo_engine::activation::{Mode, VoiceGate, db, level};
use std::time::{Duration, Instant};

#[test]
fn voice_gate_ignores_background_noise_and_preserves_word_endings() {
    let now = Instant::now();
    let mut gate = VoiceGate::default();
    let noise = level(&vec![100; 960]);
    let speech = level(&vec![4000; 960]);
    assert!(db(noise) < -35.0 && db(speech) > -35.0);
    assert!(!gate.open(Mode::Voice, false, true, noise, -35.0, now));
    assert!(gate.open(Mode::Voice, false, true, speech, -35.0, now));
    assert!(gate.open(
        Mode::Voice,
        false,
        true,
        noise,
        -35.0,
        now + Duration::from_millis(200)
    ));
    assert!(!gate.open(
        Mode::Voice,
        false,
        true,
        noise,
        -35.0,
        now + Duration::from_millis(320)
    ));
    assert!(!gate.open(Mode::Voice, false, false, speech, -35.0, now));
    assert!(!gate.open(Mode::PushToTalk, false, true, speech, -35.0, now));
    assert!(gate.open(Mode::PushToTalk, true, true, noise, -35.0, now));
}
