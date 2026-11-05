use horizon_plasmo_engine::{
    activation::Mode,
    activations::Activations,
    protocol::{self, Control},
    session::Settings,
};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
fn packet(name: &str) -> Control {
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/protocol-2.1.17.json")).unwrap();
    let s = fixtures[name].as_str().unwrap();
    let bytes: Vec<_> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect();
    protocol::read_control(&bytes).unwrap()
}
#[test]
fn inherited_channels_transmit_together_and_permission_revocation_ends_only_that_channel() {
    let Control::Config(config) = packet("config") else {
        panic!()
    };
    let mut proximity = config.activations[0].clone();
    proximity.id = 1;
    let Control::ActivationRegister(mut groups) = packet("activation_register") else {
        panic!()
    };
    groups.id = 2;
    let values = vec![proximity, groups];
    let settings = Settings::default();
    let mut engine = Activations::default();
    let now = Instant::now();
    let mut permissions = HashMap::new();
    let keys = HashSet::new();
    let plan = engine.update(&values, &settings, &permissions, &keys, true, true, 0., now);
    assert_eq!(plan.targets.len(), 2);
    assert!(plan.ended.is_empty());
    permissions.insert("pv.activation.groups".into(), false);
    let plan = engine.update(&values, &settings, &permissions, &keys, true, true, 0., now);
    assert_eq!(plan.targets.len(), 1);
    assert_eq!(plan.targets[0].id, 1);
    assert_eq!(plan.ended[0].id, 2);
    let plan = engine.update(
        &values,
        &settings,
        &permissions,
        &keys,
        false,
        true,
        0.,
        now,
    );
    assert!(plan.targets.is_empty());
    assert_eq!(plan.ended[0].id, 1);
}
#[test]
fn addon_voice_activation_preserves_its_tail_and_focus_loss_stops_every_channel() {
    let Control::ActivationRegister(mut groups) = packet("activation_register") else {
        panic!()
    };
    groups.id = 2;
    let mut settings = Settings::default();
    let saved = settings.activations.entry(2).or_default();
    saved.mode = Mode::Voice;
    saved.distance = 200;
    let values = [groups];
    let mut engine = Activations::default();
    let now = Instant::now();
    let permissions = HashMap::new();
    let keys = HashSet::new();
    assert!(
        engine
            .update(
                &values,
                &settings,
                &permissions,
                &keys,
                false,
                true,
                0.001,
                now
            )
            .targets
            .is_empty()
    );
    let plan = engine.update(
        &values,
        &settings,
        &permissions,
        &keys,
        false,
        true,
        0.2,
        now,
    );
    assert_eq!(plan.targets[0].distance, 64);
    assert_eq!(
        engine
            .update(
                &values,
                &settings,
                &permissions,
                &keys,
                false,
                true,
                0.001,
                now + Duration::from_millis(200)
            )
            .targets
            .len(),
        1
    );
    let plan = engine.update(
        &values,
        &settings,
        &permissions,
        &keys,
        false,
        false,
        0.2,
        now + Duration::from_millis(220),
    );
    assert!(plan.targets.is_empty());
    assert_eq!(plan.ended[0].id, 2);
    assert!(engine.stop().is_empty());
}
