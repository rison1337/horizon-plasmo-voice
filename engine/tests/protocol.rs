use horizon_plasmo_engine::{
    protocol::{self, Control, Datagram},
    wire::{Reader, Writer},
};

fn fixture(name: &str) -> Vec<u8> {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/protocol-2.1.17.json")).unwrap();
    let s = fixtures[name].as_str().unwrap();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn outbound_packets_match_real_plugin_serializers() {
    assert_eq!(
        protocol::player_info("26.3", &[1, 2, 3], false, true).unwrap(),
        fixture("player_info")
    );
    assert_eq!(protocol::player_end(42, 2, 16), fixture("player_end"));
    assert_eq!(
        protocol::language_request("ru_ru").unwrap(),
        fixture("language_request")
    );
    let activation = u128::from_be_bytes(fixture("activation_uuid").try_into().unwrap());
    assert_eq!(
        protocol::player_audio(1, 0, 42, &[7, 8, 9], activation, 16),
        fixture("player_audio")
    );
    assert_eq!(
        protocol::activation_distances(&[(activation, 16)]),
        fixture("activation_distances")
    );
}

#[test]
fn parses_addon_sources_and_live_registries_from_released_plugin() {
    use protocol::SourceKind;
    let Control::Source(s) = protocol::read_control(&fixture("source_static")).unwrap() else {
        panic!()
    };
    assert_eq!(s.name, "Disc");
    assert!(s.stereo);
    assert_eq!(
        s.kind,
        SourceKind::Static {
            position: [1.25, 64., -2.5],
            direction: [0., 0., 1.]
        }
    );
    let Control::Source(s) = protocol::read_control(&fixture("source_entity")).unwrap() else {
        panic!()
    };
    assert_eq!(s.kind, SourceKind::Entity(42));
    let Control::Source(s) = protocol::read_control(&fixture("source_direct")).unwrap() else {
        panic!()
    };
    assert_eq!(
        s.kind,
        SourceKind::Direct {
            relative: Some([1., 2., 3.]),
            camera_relative: true
        }
    );
    assert_eq!(s.player, 4);
    let Control::ActivationRegister(a) =
        protocol::read_control(&fixture("activation_register")).unwrap()
    else {
        panic!()
    };
    let Control::ActivationRegister(music) =
        protocol::read_control(&fixture("activation_opus")).unwrap()
    else {
        panic!()
    };
    let encoder = music.encoder.unwrap();
    assert_eq!(encoder.name, "opus");
    assert_eq!(encoder.params["mode"], "AUDIO");
    assert_eq!(encoder.params["bitrate"], "64000");
    assert!(a.dynamic_distance && a.transitive && a.stereo_supported);
    assert_eq!(a.distances, [64]);
    let Control::LineRegister(l) = protocol::read_control(&fixture("line_register")).unwrap()
    else {
        panic!()
    };
    assert_eq!(l.players.unwrap()[0].name, "Listener");
    assert_eq!(l.default_volume, 0.75);
    let Control::PlayerUpdate(p) = protocol::read_control(&fixture("player_update")).unwrap()
    else {
        panic!()
    };
    assert!(p.server_muted && p.microphone_muted && !p.voice_disabled);
    let Control::Language { name, values } = protocol::read_control(&fixture("language")).unwrap()
    else {
        panic!()
    };
    assert_eq!(name, "ru_ru");
    assert_eq!(values["pv.activation.groups"], "Группы");
    for name in [
        "activation_register",
        "activation_unregister",
        "permissions",
        "players",
        "player_update",
        "player_disconnect",
        "line_register",
        "line_unregister",
        "line_players",
        "line_player_add",
        "line_player_remove",
        "source_static",
        "source_static_no_angle",
        "source_entity",
        "source_direct",
        "source_direct_angle",
        "self_source",
        "language",
        "distance_visualize",
        "action_bar",
    ] {
        let data = fixture(name);
        assert!(
            !matches!(protocol::read_control(&data).unwrap(), Control::Unknown(_)),
            "{name}"
        );
        for end in 0..data.len() {
            assert!(
                protocol::read_control(&data[..end]).is_err(),
                "{name} truncated at {end}"
            );
        }
    }
}

#[test]
fn parses_real_plugin_configuration_and_player_source() {
    let Control::Config(config) = protocol::read_control(&fixture("config")).unwrap() else {
        panic!()
    };
    assert_eq!(config.sample_rate, 48000);
    assert_eq!(config.mtu, 1024);
    assert_eq!(config.encryption.unwrap().1, [4, 5, 6]);
    assert_eq!(config.activations[0].name, "proximity");
    assert_eq!(config.activations[0].distances, [8, 16, 32]);
    assert!(config.permissions["pv.activation.proximity"]);
    let Control::Source(source) = protocol::read_control(&fixture("source_info")).unwrap() else {
        panic!()
    };
    assert_eq!(source.id, 5);
    assert_eq!(source.player, 4);
    assert_eq!(source.name, "Игрок😀");
    assert_eq!(source.state, 1);
    assert!(!source.stereo);
    let Datagram::Audio {
        sequence,
        source,
        state,
        distance,
        data,
    } = protocol::read_datagram(&fixture("source_audio"), 1).unwrap()
    else {
        panic!()
    };
    assert_eq!((sequence, source, state, distance), (42, 5, 1, 16));
    assert_eq!(data, [7, 8, 9]);
}

#[test]
fn rejects_truncation_lengths_and_wrong_session() {
    for name in ["connection", "config", "source_info", "source_end"] {
        let bytes = fixture(name);
        for end in 0..bytes.len() {
            assert!(
                protocol::read_control(&bytes[..end]).is_err(),
                "{name} truncated at {end}"
            );
        }
    }
    let bytes = fixture("source_audio");
    for end in 0..bytes.len() {
        assert!(protocol::read_datagram(&bytes[..end], 1).is_err());
    }
    assert!(protocol::read_datagram(&bytes, 2).is_err());
    assert!(
        Reader::new(&[0xff, 0xff, 0xff, 0xff])
            .unwrap()
            .bytes(1024)
            .is_err()
    );
    assert!(protocol::read_control(&vec![0; (1 << 20) + 1]).is_err());
}

#[test]
fn java_modified_utf8_including_null_and_surrogate_pairs() {
    let expected = [
        0, 10, 0x41, 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80, 0x42,
    ];
    let mut w = Writer::default();
    w.utf("A\0😀B").unwrap();
    assert_eq!(w.0, expected);
    assert_eq!(Reader::new(&expected).unwrap().utf().unwrap(), "A\0😀B");
    for invalid in [
        &[0, 1, 0][..],
        &[0, 2, 0xc1, 0x81],
        &[0, 4, 0xf0, 0x9f, 0x98, 0x80],
    ] {
        assert!(Reader::new(invalid).unwrap().utf().is_err());
    }
}
