use crate::protocol::{Source, SourceKind};
use std::collections::HashMap;

#[derive(Clone, Default)]
pub struct Scene {
    pub listener: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub players: HashMap<u128, [f64; 3]>,
    pub entities: HashMap<i32, [f64; 3]>,
    pub directions: HashMap<u128, [f64; 3]>,
    pub occlusion: HashMap<u128, f32>,
}

/// Equal-power stereo panning; unknown/off-level speakers are inaudible.
pub fn gains(scene: &Scene, speaker: u128, distance: f32, volume: f32) -> [f32; 2] {
    let Some(pos) = scene.players.get(&speaker) else {
        return [0.0; 2];
    };
    let delta = std::array::from_fn::<_, 3, _>(|i| pos[i] - scene.listener[i]);
    if delta.iter().any(|x| !x.is_finite()) || !distance.is_finite() || distance <= 0.0 {
        return [0.0; 2];
    }
    let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt() as f32;
    let attenuation = (1.0 - length / distance).clamp(0.0, 1.0);
    let yaw = scene.yaw.to_radians();
    let side = if length > 0.01 {
        (-delta[0] as f32 * yaw.cos() - delta[2] as f32 * yaw.sin()) / length
    } else {
        0.0
    };
    let pan = side.clamp(-1.0, 1.0);
    let gain = attenuation * attenuation * volume.clamp(0.0, 2.0);
    [
        ((1.0 - pan) * 0.5).sqrt() * gain,
        ((1.0 + pan) * 0.5).sqrt() * gain,
    ]
}

pub fn source_position(scene: &Scene, source: &Source) -> Option<[f64; 3]> {
    match source.kind {
        SourceKind::Player => scene.players.get(&source.player).copied(),
        SourceKind::Entity(id) => scene.entities.get(&id).copied(),
        SourceKind::Static { position, .. } => Some(position),
        SourceKind::Direct {
            relative,
            camera_relative,
        } => {
            let [mut x, mut y, mut z] = relative.unwrap_or([0.; 3]);
            if camera_relative {
                let pitch = f64::from(scene.pitch).to_radians();
                (y, z) = (
                    y * pitch.cos() - z * pitch.sin(),
                    y * pitch.sin() + z * pitch.cos(),
                );
                let yaw = f64::from(scene.yaw).to_radians();
                (x, z) = (x * yaw.cos() - z * yaw.sin(), x * yaw.sin() + z * yaw.cos());
            }
            Some([
                scene.listener[0] + x,
                scene.listener[1] + y,
                scene.listener[2] + z,
            ])
        }
    }
}

/// Returns the two-channel gains without changing a stereo source's balance.
#[allow(clippy::too_many_arguments)]
pub fn source_gains(
    scene: &Scene,
    source: &Source,
    distance: f32,
    volume: f32,
    panning: bool,
    mono_stereo: bool,
    exponential: bool,
    directional: Option<f32>,
) -> [f32; 2] {
    let Some(position) = source_position(scene, source) else {
        return [0.; 2];
    };
    let delta: [f64; 3] = std::array::from_fn(|i| position[i] - scene.listener[i]);
    if delta.iter().any(|v| !v.is_finite()) {
        return [0.; 2];
    }
    let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt() as f32;
    let attenuation = if matches!(source.kind, SourceKind::Direct { .. }) {
        1.
    } else {
        if !distance.is_finite() || distance <= 0. {
            return [0.; 2];
        }
        let value = (1. - length / distance).clamp(0., 1.);
        if exponential { value * value } else { value }
    };
    let mut gain = attenuation * volume.clamp(0., 64.);
    let angle = if matches!(source.kind, SourceKind::Player) {
        directional.unwrap_or(0.)
    } else {
        source.angle as f32
    };
    if angle > 0. && length > 0.01 {
        let direction = match source.kind {
            SourceKind::Static { direction, .. } => Some(direction),
            _ => scene.directions.get(&source.player).copied(),
        };
        if let Some(direction) = direction {
            let norm = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
            if norm > 0.001 {
                let cosine = -delta.iter().zip(direction).map(|(a, b)| a * b).sum::<f64>()
                    / (f64::from(length) * norm);
                let edge = (f64::from(angle.clamp(1., 360.)) * 0.5).to_radians().cos();
                gain *= if cosine >= edge {
                    1.
                } else {
                    ((cosine + 1.) / (edge + 1.).max(0.0001)).clamp(0., 1.) as f32
                };
            }
        }
    }
    if !panning || (source.stereo && !mono_stereo) {
        return [gain; 2];
    }
    let yaw = scene.yaw.to_radians();
    let side = if length > 0.01 {
        (-delta[0] as f32 * yaw.cos() - delta[2] as f32 * yaw.sin()) / length
    } else {
        0.
    };
    let pan = side.clamp(-1., 1.);
    [
        ((1. - pan) * 0.5).sqrt() * gain,
        ((1. + pan) * 0.5).sqrt() * gain,
    ]
}
