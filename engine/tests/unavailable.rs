//! Run explicitly in disabled builds as well as the ordinary engine suite.
use horizon_plasmo_engine::{
    audio_runtime,
    opus::Opus,
    session::{Client, Settings},
};

#[test]
fn unavailable_audio_cannot_load_libraries_or_start_a_voice_worker() {
    if audio_runtime::prepare().is_ok() {
        return;
    }
    let Some(reason) = audio_runtime::unavailable_reason() else {
        return;
    };
    assert!(Opus::load().err().unwrap().contains(reason));
    let client = Client::start(
        "invalid.example".into(),
        || panic!("disabled voice must never initialize a key or network session"),
        |_, _| panic!("disabled voice must never initialize activations"),
        Settings::default(),
        true,
    );
    assert!(client.err().unwrap().contains(reason));
}
