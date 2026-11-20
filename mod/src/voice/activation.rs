//! Voice activity gate with a short release tail to preserve word endings.
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    PushToTalk,
    Voice,
    Inherit,
}

pub fn level(pcm: &[i16]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    (pcm.iter()
        .map(|s| (f32::from(*s) / 32768.0).powi(2))
        .sum::<f32>()
        / pcm.len() as f32)
        .sqrt()
}
pub fn db(level: f32) -> f32 {
    (20.0 * level.max(0.000001).log10()).max(-100.0)
}

#[derive(Default)]
pub struct VoiceGate {
    last_voice: Option<Instant>,
}
impl VoiceGate {
    pub fn open(
        &mut self,
        mode: Mode,
        held: bool,
        allowed: bool,
        level: f32,
        threshold: f32,
        now: Instant,
    ) -> bool {
        if !allowed {
            self.last_voice = None;
            return false;
        }
        if mode != Mode::Voice {
            self.last_voice = None;
            return held;
        }
        if db(level) >= threshold || held {
            self.last_voice = Some(now);
        }
        self.last_voice
            .is_some_and(|t| now.duration_since(t) < Duration::from_millis(300))
    }
}
