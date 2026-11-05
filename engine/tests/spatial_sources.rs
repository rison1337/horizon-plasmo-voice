use horizon_plasmo_engine::{
    occlusion,
    protocol::{self, Control},
    spatial::{self, Scene},
};
fn source(name: &str) -> protocol::Source {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/protocol-2.1.17.json")).unwrap();
    let s = fixtures[name].as_str().unwrap();
    let bytes: Vec<_> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect();
    let Control::Source(source) = protocol::read_control(&bytes).unwrap() else {
        panic!()
    };
    source
}
#[test]
fn stereo_music_preserves_channels_while_mono_conversion_tracks_the_camera() {
    let source = source("source_static");
    let mut scene = Scene {
        listener: [0., 64., 0.],
        ..Default::default()
    };
    let normal = spatial::source_gains(&scene, &source, 16., 1., true, false, true, None);
    scene.yaw = 90.;
    let rotated = spatial::source_gains(&scene, &source, 16., 1., true, false, true, None);
    assert_eq!(normal, rotated);
    assert_eq!(normal[0], normal[1]);
    assert!(normal[0] > 0.);
    let mono = spatial::source_gains(&scene, &source, 16., 1., true, true, true, None);
    assert!((mono[0] - mono[1]).abs() > 0.1);
    let linear = spatial::source_gains(&scene, &source, 16., 1., false, false, false, None);
    assert!(linear[0] > normal[0]);
}
#[test]
fn direct_audio_works_without_a_world_entity_or_proximity_distance() {
    let source = source("source_direct");
    let scene = Scene::default();
    assert_eq!(
        spatial::source_gains(&scene, &source, 0., 1., true, false, true, None),
        [1.; 2]
    );
    let entity = source_from_entity();
    assert_eq!(
        spatial::source_gains(&scene, &entity, 16., 1., true, false, true, None),
        [0.; 2]
    );
}
fn source_from_entity() -> protocol::Source {
    source("source_entity")
}
#[test]
fn wall_transmission_counts_each_crossed_voxel_once_and_excludes_both_endpoints() {
    let from = [0.5, 2.5, 0.5];
    let to = [6.5, 2.5, 0.5];
    let mut visited = Vec::new();
    let gain = occlusion::transmission(from, to, |p| {
        visited.push(p);
        if p[0] == 3 { 0.5 } else { 0. }
    });
    assert_eq!(gain, 0.5);
    assert_eq!(
        visited,
        [[1, 2, 0], [2, 2, 0], [3, 2, 0], [4, 2, 0], [5, 2, 0]]
    );
    assert_eq!(occlusion::transmission(from, to, |_| 0.), 1.);
}

#[test]
fn player_directionality_uses_the_selected_cone_even_without_a_server_angle() {
    let mut source = source("source_info");
    source.angle = 0;
    let mut scene = Scene::default();
    scene.players.insert(source.player, [0., 0., 4.]);
    scene.directions.insert(source.player, [0., 0., -1.]);
    let toward =
        spatial::source_gains(&scene, &source, 16., 1., false, false, false, Some(145.))[0];
    scene.directions.insert(source.player, [0., 0., 1.]);
    let away = spatial::source_gains(&scene, &source, 16., 1., false, false, false, Some(145.))[0];
    let disabled = spatial::source_gains(&scene, &source, 16., 1., false, false, false, None)[0];
    assert!(toward > 0.5 && away < 0.01 && disabled > 0.5);
    let full = spatial::source_gains(&scene, &source, 16., 1., false, false, false, Some(360.))[0];
    assert!((full - disabled).abs() < 0.001);
}

#[test]
fn player_volume_boosts_its_line_independently_without_a_combined_ceiling() {
    use horizon_plasmo_engine::session::Settings;
    let mut s = Settings {
        exponential_volume_slider: false,
        ..Settings::default()
    };
    let player = 42;
    let proximity = 1;
    let group = 2;
    s.line_volumes.insert(proximity, 0.5);
    s.player_volumes.insert(player, 2.);
    assert_eq!(s.source_volume(player, proximity, 1.), 1.);
    assert_eq!(s.source_volume(43, proximity, 1.), 0.5);
    assert_eq!(s.source_volume(player, group, 1.), 2.);
    s.line_volumes.insert(proximity, 2.);
    s.volume = 2.;
    s.exponential_volume_slider = true;
    let source = source("source_direct");
    let loud = s.source_volume(player, proximity, 1.);
    s.player_volumes.insert(player, 1.);
    let quiet = s.source_volume(player, proximity, 1.);
    let mix = |gain| {
        spatial::source_gains(
            &Scene::default(),
            &source,
            0.,
            gain,
            false,
            false,
            false,
            None,
        )[0]
    };
    assert_eq!(
        mix(loud) / mix(quiet),
        4.,
        "A second boost must survive spatial mixing"
    );
    s.player_muted.insert(player);
    assert_eq!(s.source_volume(player, group, 1.), 0.);
}
