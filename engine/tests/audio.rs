use horizon_plasmo_engine::{
    audio::Resampler,
    crypto::AudioCipher,
    jitter::{Jitter, Playout},
    opus::Opus,
    spatial::{self, Scene},
};
use std::time::{Duration, Instant};

#[test]
fn cipher_matches_independent_openssl_vector_and_rejects_bad_padding() {
    // openssl enc -aes-128-cbc -K 000102030405060708090a0b0c0d0e0f
    // -iv 101112131415161718191a1b1c1d1e1f; plaintext "Plasmo Voice".
    let cipher = AudioCipher::new(&(0..16).collect::<Vec<u8>>()).unwrap();
    let iv = std::array::from_fn(|i| 16 + i as u8);
    let encrypted = cipher.encrypt_with_iv(b"Plasmo Voice", iv).unwrap();
    assert_eq!(&encrypted[16..], &hex("d46e786971e2129985bca1a408e91794"));
    assert_eq!(cipher.decrypt(&encrypted).unwrap(), b"Plasmo Voice");
    let mut broken = encrypted;
    broken[15] ^= 0xff;
    assert!(cipher.decrypt(&broken).is_err());
    assert!(cipher.decrypt(&[0; 16]).is_err());
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn opus_survives_encryption_reordering_and_a_lost_frame() {
    let opus = Opus::load().expect("install libopus to run the codec integration test");
    let mut encoder = opus.encoder(48000).unwrap();
    let mut decoder = opus.decoder(false).unwrap();
    let cipher = AudioCipher::new(&[7; 16]).unwrap();
    let mut packets = Vec::new();
    for frame in 0..8 {
        let pcm: Vec<i16> = (0..960)
            .map(|i| {
                ((2.0 * std::f32::consts::PI * 440.0 * (frame * 960 + i) as f32 / 48000.0).sin()
                    * 12000.0) as i16
            })
            .collect();
        packets.push(
            cipher
                .encrypt(&encoder.encode(&pcm, 1000).unwrap())
                .unwrap(),
        );
    }
    let now = Instant::now();
    let mut jitter = Jitter::default();
    for index in [1, 0, 3, 2, 5, 7, 6] {
        jitter.push(index as u64, packets[index].clone(), now);
    }
    assert!(matches!(jitter.pop(now), Playout::Idle));
    let mut energy = 0.0;
    for frame in 0..8 {
        let at = now + Duration::from_millis(40 + frame * 20);
        let pcm = match jitter.pop(at) {
            Playout::Packet(bytes) => decoder
                .decode(Some(&cipher.decrypt(&bytes).unwrap()))
                .unwrap(),
            Playout::Lost => decoder.decode(None).unwrap(),
            Playout::Idle => panic!("unexpected idle"),
        };
        assert_eq!(pcm.len(), 960);
        energy += pcm.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>();
    }
    assert!(energy > 1e9);
}

#[test]
fn duplicate_packets_do_not_evict_queued_speech() {
    let now = Instant::now();
    let mut jitter = Jitter::default();
    for sequence in 0..12 {
        jitter.push(sequence, vec![sequence as u8], now);
    }
    jitter.push(11, vec![11], now);
    for sequence in 0..12 {
        match jitter.pop(now + Duration::from_millis(40 + sequence * 20)) {
            Playout::Packet(bytes) => assert_eq!(bytes, [sequence as u8]),
            _ => panic!("duplicate UDP packet displaced queued speech"),
        }
    }
}

#[test]
fn resampling_is_continuous_across_arbitrary_callbacks() {
    let input: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.03).sin()).collect();
    let whole = Resampler::new(44100, 48000, 1).push(&input);
    let mut split = Resampler::new(44100, 48000, 1);
    let pieces: Vec<f32> = input.chunks(137).flat_map(|c| split.push(c)).collect();
    assert!(whole.len().abs_diff(48000) <= 2);
    assert_eq!(whole.len(), pieces.len());
    assert!(
        whole
            .iter()
            .zip(pieces)
            .all(|(a, b)| (a - b).abs() < 0.0001)
    );
}

#[test]
fn spatial_audio_tracks_distance_camera_rotation_and_absent_players() {
    let mut scene = Scene::default();
    assert_eq!(spatial::gains(&scene, 1, 16.0, 1.0), [0.0; 2]);
    scene.players.insert(1, [-4.0, 0.0, 0.0]);
    let right = spatial::gains(&scene, 1, 16.0, 1.0);
    assert!(right[1] > right[0]);
    scene.yaw = 180.0;
    let left = spatial::gains(&scene, 1, 16.0, 1.0);
    assert!(left[0] > left[1]);
    scene.players.insert(1, [100.0, 0.0, 0.0]);
    assert_eq!(spatial::gains(&scene, 1, 16.0, 1.0), [0.0; 2]);
}

#[test]
fn server_opus_bitrate_changes_encoded_audio_budget() {
    use horizon_plasmo_engine::protocol::Codec;
    let opus = Opus::load().unwrap();
    let pcm: Vec<i16> = (0..960)
        .map(|i| ((i as f32 * 0.0576).sin() * 14000.) as i16)
        .collect();
    let mut sizes = Vec::new();
    for bitrate in [12000, 96000] {
        let config = Codec {
            name: "opus".into(),
            params: [
                ("mode".into(), "AUDIO".into()),
                ("bitrate".into(), bitrate.to_string()),
            ]
            .into(),
        };
        let mut encoder = opus.configured_encoder(48000, Some(&config)).unwrap();
        let total: usize = (0..20)
            .map(|_| encoder.encode(&pcm, 1275).unwrap().len())
            .sum();
        sizes.push(total);
    }
    assert!(sizes[1] > sizes[0] * 2, "{sizes:?}");
}
#[test]
fn adaptive_buffer_waits_for_variable_arrivals_and_resets_after_a_pause() {
    let now = Instant::now();
    let mut fixed = Jitter::default();
    fixed.configure(1, false);
    let mut adaptive = Jitter::default();
    adaptive.configure(1, true);
    for buffer in [&mut fixed, &mut adaptive] {
        buffer.push(0, vec![1], now);
        buffer.push(1, vec![2], now + Duration::from_millis(100));
    }
    assert!(matches!(fixed.pop(now), Playout::Packet(_)));
    assert!(matches!(adaptive.pop(now), Playout::Idle));
    assert!(matches!(
        adaptive.pop(now + Duration::from_millis(100)),
        Playout::Packet(_)
    ));
    adaptive.push(3, vec![3], now + Duration::from_secs(2));
    assert!(matches!(adaptive.pop(now+Duration::from_secs(2)),Playout::Packet(v)if v==[3]));
}
#[test]
fn compressor_reduces_loud_speech_and_enforces_the_peak_ceiling() {
    use horizon_plasmo_engine::audio::Dynamics;
    let mut dynamics = Dynamics::default();
    let mut loud = vec![30000i16; 48000];
    dynamics.process(&mut loud, 48000, -10., -6.);
    assert!(loud.iter().all(|v| v.abs() <= 16423));
    assert!(loud[47999] < 15000);
    let mut quiet = vec![1000i16; 48000];
    dynamics.process(&mut quiet, 48000, -10., -6.);
    assert!((quiet[47999] - 1000).abs() < 30);
}
