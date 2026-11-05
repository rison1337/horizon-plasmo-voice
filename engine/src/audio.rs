//! Bounded audio queues. Device callbacks never wait for the game or network.
use crate::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
};

pub fn devices(input: bool) -> Vec<String> {
    let host = cpal::default_host();
    let devices = if input {
        host.input_devices()
    } else {
        host.output_devices()
    };
    devices
        .map(|d| d.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}
fn device(input: bool, name: &str) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if name.is_empty() {
        return (if input {
            host.default_input_device()
        } else {
            host.default_output_device()
        })
        .ok_or("no default audio device".into());
    }
    let mut available = if input {
        host.input_devices()
    } else {
        host.output_devices()
    }
    .map_err(|e| e.to_string())?;
    available
        .find(|d| d.name().is_ok_and(|n| n == name))
        .ok_or("configured audio device is unavailable".into())
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
    _stream: cpal::Stream,
    rx: Receiver<Vec<f32>>,
    pub gate: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    resampler: Resampler,
    pending: VecDeque<i16>,
    frame: usize,
}
impl Capture {
    pub fn open(name: &str, rate: u32) -> Result<Self> {
        Self::open_options(name, rate, false)
    }
    pub fn open_options(name: &str, rate: u32, stereo_capture: bool) -> Result<Self> {
        let d = device(true, name)?;
        let config = d.default_input_config().map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel(8);
        let gate = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => capture::<f32>(
                &d,
                &config.clone().into(),
                tx,
                gate.clone(),
                failed.clone(),
                stereo_capture,
            ),
            cpal::SampleFormat::I16 => capture::<i16>(
                &d,
                &config.clone().into(),
                tx,
                gate.clone(),
                failed.clone(),
                stereo_capture,
            ),
            cpal::SampleFormat::U16 => capture::<u16>(
                &d,
                &config.clone().into(),
                tx,
                gate.clone(),
                failed.clone(),
                stereo_capture,
            ),
            _ => return Err("unsupported microphone sample format".into()),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            rx,
            gate,
            failed,
            resampler: Resampler::new(config.sample_rate().0, rate, 1),
            pending: VecDeque::new(),
            frame: rate as usize / 50,
        })
    }
    pub fn set_transmitting(&mut self, yes: bool) {
        let before = self.gate.swap(yes, Ordering::Relaxed);
        if before != yes {
            while self.rx.try_recv().is_ok() {}
            self.pending.clear();
            self.resampler.data.clear();
            self.resampler.at = 0.0;
        }
    }
    pub fn frames(&mut self, gain: f32) -> Vec<Vec<i16>> {
        if !self.gate.load(Ordering::Relaxed) {
            return Vec::new();
        }
        for input in self.rx.try_iter().take(8) {
            self.pending.extend(
                self.resampler
                    .push(&input)
                    .iter()
                    .map(|s| (s * gain * 32767.0).clamp(-32768.0, 32767.0) as i16),
            );
        }
        // Discard latency after a stall, rather than transmitting old speech.
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
        self.failed.load(Ordering::Relaxed)
    }
}
fn capture<T>(
    d: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: SyncSender<Vec<f32>>,
    gate: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    stereo_capture: bool,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = usize::from(config.channels);
    d.build_input_stream(
        config,
        move |data: &[T], _| {
            if !gate.load(Ordering::Relaxed) {
                return;
            }
            let mono = data
                .chunks_exact(channels)
                .map(|f| {
                    if stereo_capture {
                        f.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() / channels as f32
                    } else {
                        f[0].to_sample::<f32>()
                    }
                })
                .collect();
            let _ = tx.try_send(mono);
        },
        move |_| {
            failed.store(true, Ordering::Relaxed);
        },
        None,
    )
    .map_err(|e| e.to_string())
}

pub struct Output {
    _stream: cpal::Stream,
    tx: SyncSender<Vec<f32>>,
    failed: Arc<AtomicBool>,
    resampler: Resampler,
}
impl Output {
    pub fn open(name: &str) -> Result<Self> {
        Self::buffered(name, 5)
    }
    pub fn buffered(name: &str, buffers: u8) -> Result<Self> {
        let d = device(false, name)?;
        let config = d.default_output_config().map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::sync_channel(usize::from(buffers.clamp(2, 12)));
        let failed = Arc::new(AtomicBool::new(false));
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                output::<f32>(&d, &config.clone().into(), rx, failed.clone())
            }
            cpal::SampleFormat::I16 => {
                output::<i16>(&d, &config.clone().into(), rx, failed.clone())
            }
            cpal::SampleFormat::U16 => {
                output::<u16>(&d, &config.clone().into(), rx, failed.clone())
            }
            _ => return Err("unsupported output sample format".into()),
        }?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            _stream: stream,
            tx,
            failed,
            resampler: Resampler::new(48000, config.sample_rate().0, 2),
        })
    }
    pub fn write(&mut self, stereo: &[f32]) {
        let _ = self.tx.try_send(self.resampler.push(stereo));
    }
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}
fn output<T>(
    d: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<Vec<f32>>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut pending = VecDeque::<f32>::new();
    d.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for buffer in rx.try_iter().take(4) {
                pending.extend(buffer);
            }
            for frame in data.chunks_mut(channels) {
                let left = pending.pop_front().unwrap_or(0.0).clamp(-1.0, 1.0);
                let right = pending.pop_front().unwrap_or(0.0).clamp(-1.0, 1.0);
                for (c, sample) in frame.iter_mut().enumerate() {
                    let v = if channels == 1 {
                        (left + right) * 0.5
                    } else {
                        match c {
                            0 => left,
                            1 => right,
                            _ => 0.0,
                        }
                    };
                    *sample = T::from_sample(v);
                }
            }
        },
        move |_| {
            failed.store(true, Ordering::Relaxed);
        },
        None,
    )
    .map_err(|e| e.to_string())
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
