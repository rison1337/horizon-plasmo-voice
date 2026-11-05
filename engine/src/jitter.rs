//! Bounded packet reordering with a 40 ms prebuffer and finite gap recovery.
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

pub enum Playout {
    Packet(Vec<u8>),
    Lost,
    Idle,
}
pub struct Jitter {
    packets: BTreeMap<u64, Vec<u8>>,
    next: Option<u64>,
    since: Option<Instant>,
    last: Option<Instant>,
    started: bool,
    delay: u64,
    adaptive: bool,
    variance_ms: f64,
    arrival: Option<(u64, Instant)>,
}
impl Default for Jitter {
    fn default() -> Self {
        Self {
            packets: BTreeMap::new(),
            next: None,
            since: None,
            last: None,
            started: false,
            delay: 40,
            adaptive: false,
            variance_ms: 0.,
            arrival: None,
        }
    }
}
impl Jitter {
    pub fn configure(&mut self, frames: u8, adaptive: bool) {
        self.delay = u64::from(frames.clamp(1, 10).saturating_sub(1)) * 20;
        self.adaptive = adaptive;
    }
    pub fn push(&mut self, seq: u64, data: Vec<u8>, now: Instant) {
        if self
            .last
            .is_some_and(|t| now.duration_since(t) > Duration::from_millis(500))
        {
            self.end();
        }
        if self.started && self.next.is_some_and(|next| seq < next) {
            return;
        }
        if self.packets.contains_key(&seq) {
            return;
        }
        if let Some((previous, at)) = self.arrival
            && seq > previous
        {
            let expected = seq.saturating_sub(previous).min(100) as f64 * 20.;
            let actual = now.saturating_duration_since(at).as_secs_f64() * 1000.;
            self.variance_ms += ((actual - expected).abs() - self.variance_ms) / 16.;
        }
        if self.arrival.is_none_or(|(previous, _)| seq > previous) {
            self.arrival = Some((seq, now));
        }
        if !self.started {
            self.next = Some(self.next.map_or(seq, |n| n.min(seq)));
        }
        if self.packets.len() >= 12 {
            self.packets.pop_first();
        }
        self.packets.entry(seq).or_insert(data);
        self.since.get_or_insert(now);
        self.last = Some(now);
    }
    pub fn pop(&mut self, now: Instant) -> Playout {
        if self.packets.is_empty()
            && self
                .last
                .is_none_or(|t| now.duration_since(t) > Duration::from_millis(150))
        {
            return Playout::Idle;
        }
        if !self.started {
            if self.since.is_none_or(|t| {
                now.duration_since(t)
                    < Duration::from_millis(if self.adaptive {
                        self.delay.max((self.variance_ms * 4.).min(160.) as u64)
                    } else {
                        self.delay
                    })
            }) {
                return Playout::Idle;
            }
            self.started = true;
        }
        let Some(mut next) = self.next else {
            return Playout::Idle;
        };
        if let Some((&first, _)) = self.packets.first_key_value()
            && first > next.saturating_add(10)
        {
            next = first;
        }
        self.next = Some(next.wrapping_add(1));
        match self.packets.remove(&next) {
            Some(p) => Playout::Packet(p),
            None => Playout::Lost,
        }
    }
    pub fn end(&mut self) {
        let delay = self.delay;
        let adaptive = self.adaptive;
        *self = Self {
            delay,
            adaptive,
            ..Self::default()
        };
    }
}
