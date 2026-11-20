//! Audio devices use the same verified OpenAL build as the HRTF renderer.
//! Bounded playback buffers and capture reads live on the voice worker; the
//! render thread never opens a device or waits for an audio callback.
use super::device_api::{Device, Ptr, api};
use crate::Result;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub fn devices(input: bool) -> Vec<String> {
    api().map(|a| a.devices(input)).unwrap_or_default()
}

/// A continuous linear resampler, including interpolation across callback boundaries.
pub struct Resampler {
    channels: usize,
    ratio: f64,
    at: f64,
    data: VecDeque<f32>,
}
impl Resampler {
    pub fn new(from: u32, to: u32, channels: usize) -> Self {
        Self {
            channels,
            ratio: from as f64 / to as f64,
            at: 0.0,
            data: VecDeque::new(),
        }
    }
    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        self.data.extend(input);
        let mut out = Vec::new();
        while (self.at.floor() as usize + 1) * self.channels < self.data.len() {
            let frame = self.at.floor() as usize;
            let t = self.at.fract() as f32;
            for c in 0..self.channels {
                let a = self.data[frame * self.channels + c];
                let b = self.data[(frame + 1) * self.channels + c];
                out.push(a + (b - a) * t);
            }
            self.at += self.ratio;
        }
        let consumed = (self.at.floor() as usize).min(self.data.len() / self.channels);
        self.data.drain(..consumed * self.channels);
        self.at -= consumed as f64;
        out
    }
}

pub struct Capture {
    device: Device,
    pub gate: Arc<AtomicBool>,
    channels: usize,
    pending: VecDeque<i16>,
    frame: usize,
}
impl Capture {
    pub fn open(name: &str, rate: u32) -> Result<Self> {
        Self::open_options(name, rate, false)
    }
    pub fn open_options(name: &str, rate: u32, stereo: bool) -> Result<Self> {
        let api = api()?;
        let (device, channels) = api.capture(name, rate, stereo)?;
        Ok(Self {
            device,
            channels,
            gate: Arc::new(AtomicBool::new(false)),
            pending: VecDeque::new(),
            frame: rate as usize / 50,
        })
    }
    pub fn set_transmitting(&mut self, yes: bool) {
        let before = self.gate.swap(yes, Ordering::Relaxed);
        if before != yes {
            self.pending.clear();
            // Discard queued capture both when starting and stopping, so an
            // old frame cannot leak into the next PTT activation.
            let _ = self.device.capture_read(self.channels);
            self.device.capture_active(yes);
        }
    }
    pub fn frames(&mut self, gain: f32) -> Vec<Vec<i16>> {
        if !self.gate.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let data = self.device.capture_read(self.channels);
        for frame in data.chunks_exact(self.channels) {
            let mono = frame.iter().map(|s| f32::from(*s)).sum::<f32>() / self.channels as f32;
            self.pending
                .push_back((mono * gain).clamp(-32768., 32767.) as i16);
        }
        if self.pending.len() > self.frame * 5 {
            self.pending.drain(..self.pending.len() - self.frame * 2);
        }
        let mut frames = Vec::new();
        while self.pending.len() >= self.frame {
            frames.push(self.pending.drain(..self.frame).collect());
        }
        frames
    }
    pub fn failed(&self) -> bool {
        self.device.failed()
    }
}

pub struct Output {
    device: Device,
    context: Ptr,
    source: u32,
    free: Vec<u32>,
    buffers: Vec<u32>,
}
impl Output {
    pub fn open(name: &str) -> Result<Self> {
        Self::buffered(name, 5)
    }
    pub fn buffered(name: &str, buffers: u8) -> Result<Self> {
        let device = api()?.output(name)?;
        let context = device.context()?;
        let api = api()?;
        api.current(context)?;
        let mut source = 0;
        let mut buffers = vec![0; usize::from(buffers.clamp(2, 12))];
        unsafe {
            (api.gen_sources)(1, &mut source);
            (api.gen_buffers)(buffers.len() as i32, buffers.as_mut_ptr());
            (api.source_i)(source, 0x202, 1);
            (api.source_f)(source, 0x1021, 0.);
        }
        api.check()?;
        Ok(Self {
            device,
            context,
            source,
            free: buffers.clone(),
            buffers,
        })
    }
    pub fn write(&mut self, stereo: &[f32]) {
        let Ok(api) = api() else { return };
        if api.current(self.context).is_err() {
            return;
        }
        let mut done = 0;
        unsafe {
            (api.get_source_i)(self.source, 0x1016, &mut done);
        }
        for _ in 0..done.min(self.buffers.len() as i32) {
            let mut id = 0;
            unsafe {
                (api.unqueue)(self.source, 1, &mut id);
            }
            self.free.push(id);
        }
        let Some(id) = self.free.pop() else { return };
        let pcm: Vec<i16> = stereo
            .iter()
            .take(48000)
            .map(|v| (v.clamp(-1., 1.) * 32767.) as i16)
            .collect();
        unsafe {
            (api.data)(
                id,
                0x1103,
                pcm.as_ptr().cast(),
                (pcm.len() * 2) as i32,
                48000,
            );
            (api.queue)(self.source, 1, &id);
        }
        let mut state = 0;
        unsafe {
            (api.get_source_i)(self.source, 0x1010, &mut state);
            if state != 0x1012 {
                (api.play)(self.source);
            }
        }
    }
    pub fn failed(&self) -> bool {
        self.device.failed()
            || api().is_ok_and(|api| api.current(self.context).and_then(|_| api.check()).is_err())
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        if let Ok(api) = api() {
            let _ = api.current(self.context);
            unsafe {
                (api.stop)(self.source);
                (api.delete_sources)(1, &self.source);
                (api.delete_buffers)(self.buffers.len() as i32, self.buffers.as_ptr());
                (api.set_current)(std::ptr::null_mut());
                (api.destroy)(self.context);
            }
        }
    }
}

/// Stateful soft-knee compressor with a final peak ceiling, before Opus encoding.
#[derive(Default)]
pub struct Dynamics {
    envelope: f32,
}
impl Dynamics {
    pub fn process(&mut self, pcm: &mut [i16], rate: u32, threshold_db: f32, ceiling_db: f32) {
        let attack = (-1. / (rate as f32 * 0.002)).exp();
        let release = (-1. / (rate as f32 * 0.15)).exp();
        let ceiling = 10f32.powf(ceiling_db / 20.);
        for sample in pcm {
            let value = f32::from(*sample) / 32768.;
            let magnitude = value.abs();
            let coefficient = if magnitude > self.envelope {
                attack
            } else {
                release
            };
            self.envelope = coefficient * self.envelope + (1. - coefficient) * magnitude;
            let db = 20. * self.envelope.max(1e-6).log10();
            let over = (db - threshold_db).max(0.);
            let gain = 10f32.powf(-over * 0.75 / 20.);
            *sample = ((value * gain).clamp(-ceiling, ceiling) * 32768.) as i16;
        }
    }
}
