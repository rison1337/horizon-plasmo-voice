use horizon_plasmo_engine::{
    protocol::{self, Control},
    session::{Client, Event, KeyPair, Settings, Status},
    wire::Writer,
};
use std::{
    net::UdpSocket,
    thread,
    time::{Duration, Instant},
};

struct TestKey;
impl KeyPair for TestKey {
    fn public_der(&self) -> Vec<u8> {
        vec![1, 2, 3]
    }
    fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>, String> {
        assert_eq!(encrypted, [4, 5, 6]);
        Ok(vec![7; 16])
    }
}
fn test_key() -> Result<Box<dyn KeyPair>, String> {
    Ok(Box::new(TestKey))
}
fn wait(client: &Client, mut matching: impl FnMut(&Event) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        for event in client.events() {
            if let Event::Error(error) = &event {
                panic!("worker error: {error}");
            }
            if matching(&event) {
                return;
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("worker event timeout");
}
#[test]
fn negotiates_over_control_channel_and_connects_real_udp_socket() {
    let server = UdpSocket::bind("127.0.0.1:0").unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut settings = Settings {
        compressor_limiter: false,
        panning: false,
        exponential_volume_slider: false,
        ..Settings::default()
    };
    let client = Client::start(
        "127.0.0.1".into(),
        test_key,
        |_, _| 2,
        settings.clone(),
        false,
    )
    .unwrap();
    assert!(client.control(&[2]));
    wait(
        &client,
        |event| matches!(event,Event::Tcp(bytes) if *bytes == protocol::player_info("26.3",&[1,2,3],false,false).unwrap()),
    );
    let mut connection = Writer::default();
    connection.u8(1);
    connection.uuid(1);
    connection.utf("0.0.0.0").unwrap();
    connection.i32(server.local_addr().unwrap().port() as i32);
    assert!(matches!(
        protocol::read_control(&connection.0),
        Ok(Control::Connection { .. })
    ));
    client.control(&connection.0);
    let mut buf = [0; 8192];
    let (n, peer) = server.recv_from(&mut buf).unwrap();
    assert_eq!(&buf[..5], &[0x4e, 0x90, 0x04, 0xe9, 1]);
    assert!(n > 29);
    server
        .send_to(&protocol::ping(1, 0, None).unwrap(), peer)
        .unwrap();
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/protocol-2.1.17.json")).unwrap();
    let config = fixtures["config"].as_str().unwrap();
    let bytes: Vec<u8> = (0..config.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&config[i..i + 2], 16).unwrap())
        .collect();
    client.control(&bytes);
    wait(&client, |event| {
        matches!(event, Event::Status(Status::Connected))
    });
    // An unknown source is requested over reliable control, not decoded with
    // a guessed format or shared decoder. Repeated UDP packets are throttled.
    let mut packet = Writer::default();
    packet.i32(protocol::MAGIC as i32);
    packet.u8(3);
    packet.uuid(1);
    packet.u64(0);
    packet.u64(0);
    packet.bytes(&[0; 32]);
    packet.uuid(123);
    packet.u8(1);
    packet.u16(16);
    for _ in 0..8 {
        server.send_to(&packet.0, peer).unwrap();
    }
    wait(
        &client,
        |event| matches!(event,Event::Tcp(bytes) if *bytes == protocol::source_request(123)),
    );
    thread::sleep(Duration::from_millis(30));
    assert!(
        !client.events().any(
            |event| matches!(event,Event::Tcp(bytes) if bytes == protocol::source_request(123))
        )
    );

    // A known source can change its format/state while staying at the same UUID.
    // Ask for its new metadata rather than dropping that speaker forever.
    let source = fixtures["source_info"].as_str().unwrap();
    let source: Vec<u8> = (0..source.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&source[i..i + 2], 16).unwrap())
        .collect();
    client.control(&source);
    client.control(&[2]);
    wait(
        &client,
        |event| matches!(event, Event::Tcp(bytes) if bytes.first() == Some(&10)),
    );
    let source_id_offset = packet.0.len() - 19;
    packet.0[source_id_offset..source_id_offset + 16].copy_from_slice(&5u128.to_be_bytes());
    server.send_to(&packet.0, peer).unwrap();
    thread::sleep(Duration::from_millis(30));
    assert!(
        !client.events().any(
            |event| matches!(event, Event::Tcp(bytes) if bytes == protocol::source_request(5))
        )
    );
    packet.0[source_id_offset + 16] = 2;
    server.send_to(&packet.0, peer).unwrap();
    wait(
        &client,
        |event| matches!(event, Event::Tcp(bytes) if *bytes == protocol::source_request(5)),
    );

    // A real encrypted Opus stream on a non-positional direct source must
    // remain audible with distance=0, even with no entity in the local scene.
    let source = fixtures["source_direct"].as_str().unwrap();
    let bytes: Vec<_> = (0..source.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&source[i..i + 2], 16).unwrap())
        .collect();
    let Control::Source(direct_source) = protocol::read_control(&bytes).unwrap() else {
        panic!()
    };
    client.control(&bytes);
    let opus = horizon_plasmo_engine::opus::Opus::load().unwrap();
    let mut encoder = opus.encoder(48000).unwrap();
    let cipher = horizon_plasmo_engine::crypto::AudioCipher::new(&[7; 16]).unwrap();
    let pcm: Vec<_> = (0..960)
        .map(|i| ((i as f32 * 0.0576).sin() * 12000.) as i16)
        .collect();
    let mut audible = false;
    for sequence in 100..112 {
        let data = cipher
            .encrypt(&encoder.encode(&pcm, 1000).unwrap())
            .unwrap();
        let mut down = Writer::default();
        down.i32(protocol::MAGIC as i32);
        down.u8(3);
        down.uuid(1);
        down.u64(0);
        down.u64(sequence);
        down.bytes(&data);
        down.uuid(8);
        down.u8(4);
        down.u16(0);
        server.send_to(&down.0, peer).unwrap();
        thread::sleep(Duration::from_millis(20));
        if let Some(snapshot) = client.snapshot() {
            audible |= snapshot.output_level > 0.01
                && snapshot.speakers.iter().any(|s| s.source == 8 && s.audible);
        }
    }
    assert!(
        audible,
        "Direct source was not decrypted, decoded and mixed"
    );
    // Exercise the entire real UDP/AES/Opus/mix path at a quiet input level,
    // so a compressor or final limiter cannot disguise a gain ceiling.
    let quiet_pcm: Vec<_> = (0..960)
        .map(|i| ((i as f32 * 0.0576).sin() * 500.) as i16)
        .collect();
    // Encoding time must not slow the simulated server's 20 ms packet clock.
    // Scalar/debug builds on CI can otherwise drift behind playout and measure
    // packet-loss concealment rather than the configured volume multiplier.
    let mut packets = (0..36 * 5)
        .map(|_| {
            cipher
                .encrypt(&encoder.encode(&quiet_pcm, 1000).unwrap())
                .unwrap()
        })
        .collect::<Vec<_>>()
        .into_iter();
    let mut sequence = 200;
    let mut measure = |settings: &Settings| {
        assert!(client.settings(settings.clone()));
        let mut levels = Vec::new();
        let start = Instant::now();
        for n in 0..36 {
            let data = packets.next().unwrap();
            let mut down = Writer::default();
            down.i32(protocol::MAGIC as i32);
            down.u8(3);
            down.uuid(1);
            down.u64(0);
            down.u64(sequence);
            sequence += 1;
            down.bytes(&data);
            down.uuid(8);
            down.u8(4);
            down.u16(0);
            server.send_to(&down.0, peer).unwrap();
            let deadline = start + Duration::from_millis((n + 1) * 20);
            thread::sleep(deadline.saturating_duration_since(Instant::now()));
            if let Some(s) = client.snapshot()
                && n > 16
                && s.output_level > 0.00001
            {
                levels.push(s.output_level);
            }
        }
        assert!(
            levels.len() >= 3,
            "Not enough decoded audio to measure gain"
        );
        // Scheduling jitter (notably Wine's timer granularity) can include
        // under-run/concealment frames. The steady decoded signal is periodic;
        // its median RMS measures gain without bias from a few late frames.
        levels.sort_by(f32::total_cmp);
        levels[levels.len() / 2]
    };
    settings.line_volumes.insert(direct_source.line, 0.5);
    let half = measure(&settings);
    settings.player_volumes.insert(direct_source.player, 2.);
    let boosted = measure(&settings);
    assert!(
        (boosted / half - 2.).abs() < 0.2,
        "player boost: {half} -> {boosted}"
    );
    // A different player/line cannot overwrite the selected person's level.
    settings.line_volumes.insert(direct_source.line ^ 1, 0.);
    settings.player_volumes.insert(direct_source.player ^ 1, 0.);
    let unchanged = measure(&settings);
    assert!((unchanged / boosted - 1.).abs() < 0.1);
    settings.exponential_volume_slider = true;
    settings.line_volumes.insert(direct_source.line, 2.);
    settings.player_volumes.insert(direct_source.player, 1.);
    let high = measure(&settings);
    settings.player_volumes.insert(direct_source.player, 2.);
    let higher = measure(&settings);
    assert!(
        (higher / high - 4.).abs() < 0.4,
        "combined gain ceiling: {high} -> {higher}"
    );
    println!(
        "Decoded RMS: line 50%={half}, player 200%={boosted}; exponential high={high}, boosted={higher}"
    );
    drop(client);
    thread::sleep(Duration::from_millis(20));
}
