//! Independent gates for server-provided capture channels, including inheritance.
use crate::{
    activation::{Mode, VoiceGate},
    protocol::Activation,
    session::Settings,
};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub id: u128,
    pub distance: u16,
}
#[derive(Default)]
pub struct Plan {
    pub targets: Vec<Target>,
    pub ended: Vec<Target>,
}
#[derive(Default)]
pub struct Activations {
    gates: HashMap<u128, VoiceGate>,
    live: HashMap<u128, u16>,
}
pub fn mode(a: &Activation, s: &Settings) -> Mode {
    s.activations.get(&a.id).map_or(
        if a.name == "proximity" {
            s.mode
        } else {
            Mode::Inherit
        },
        |v| v.mode,
    )
}
pub fn distance(a: &Activation, s: &Settings) -> u16 {
    let request = s
        .activations
        .get(&a.id)
        .map_or(if a.name == "proximity" { s.distance } else { 0 }, |v| {
            v.distance
        });
    if request == 0 {
        return a.default_distance;
    }
    if a.dynamic_distance {
        request.clamp(1, a.distances.last().copied().unwrap_or(32767))
    } else {
        a.distances
            .iter()
            .copied()
            .min_by_key(|v| v.abs_diff(request))
            .unwrap_or(a.default_distance)
    }
}
impl Activations {
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        values: &[Activation],
        s: &Settings,
        permissions: &HashMap<String, bool>,
        keys: &HashSet<u128>,
        ptt: bool,
        allowed: bool,
        level: f32,
        now: Instant,
    ) -> Plan {
        let permitted = |a: &Activation| {
            allowed && permissions.get(&format!("pv.activation.{}", a.name)) != Some(&false)
        };
        let mut inherited = false;
        if let Some(proximity) = values.iter().find(|a| a.name == "proximity") {
            let selected = mode(proximity, s);
            inherited = self.gates.entry(proximity.id).or_default().open(
                if selected == Mode::Inherit {
                    Mode::PushToTalk
                } else {
                    selected
                },
                ptt || keys.contains(&proximity.id),
                permitted(proximity),
                level,
                s.threshold_db,
                now,
            );
        }
        let mut current = HashMap::new();
        for a in values {
            let selected = mode(a, s);
            let active = if a.name == "proximity" {
                inherited
            } else if selected == Mode::Inherit {
                permitted(a) && (inherited || keys.contains(&a.id))
            } else {
                self.gates.entry(a.id).or_default().open(
                    selected,
                    keys.contains(&a.id),
                    permitted(a),
                    level,
                    s.threshold_db,
                    now,
                )
            };
            if active {
                current.insert(a.id, distance(a, s));
            }
        }
        let targets = current
            .iter()
            .map(|(&id, &distance)| Target { id, distance })
            .collect();
        let ended = self
            .live
            .iter()
            .filter(|(id, distance)| current.get(id) != Some(distance))
            .map(|(&id, &distance)| Target { id, distance })
            .collect();
        self.live = current;
        self.gates
            .retain(|id, _| values.iter().any(|a| a.id == *id));
        Plan { targets, ended }
    }
    pub fn stop(&mut self) -> Vec<Target> {
        self.gates.clear();
        self.live
            .drain()
            .map(|(id, distance)| Target { id, distance })
            .collect()
    }
    pub fn wants_capture(
        values: &[Activation],
        s: &Settings,
        keys: &HashSet<u128>,
        ptt: bool,
    ) -> bool {
        ptt || !keys.is_empty() || values.iter().any(|a| mode(a, s) == Mode::Voice)
    }
}
