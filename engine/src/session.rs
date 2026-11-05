//! Session-owned UDP, Opus and devices on one worker. Main-thread work is bounded.
use crate::{
    Result,
    activation::{self, Mode},
    activations::Activations,
    audio::{Capture, Output, Resampler},
    crypto::AudioCipher,
    denoise::Denoiser,
    jitter::{Jitter, Playout},
    opus::{Decoder, Encoder, Opus},
    protocol::{self, Control, Datagram, Source},
    spatial::{self, Scene},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    net::{ToSocketAddrs, UdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub trait KeyPair: Send {
    fn public_der(&self) -> Vec<u8>;
    fn decrypt(&self, encrypted: &[u8]) -> Result<Vec<u8>>;
}
pub type KeyFactory = fn() -> Result<Box<dyn KeyPair>>;
pub type ActivationId = fn(&str, &str) -> u128;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOverlap {
    MuteDirect,
    #[default]
    MuteProximity,
    Off,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IconVisibility {
    #[default]
    GuiOnly,
    Always,
    Off,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayStyle {
    #[default]
    NameSkin,
    Name,
    Skin,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayMode {
    #[default]
    Off,
    Talking,
    Always,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    #[default]
    BottomCenter,
    BottomRight,
}
impl Anchor {
    pub fn point(self, width: f32, height: f32, w: f32, h: f32) -> [f32; 2] {
        let (x, y) = match self {
            Self::TopLeft => (0, 0),
            Self::TopCenter => (1, 0),
            Self::TopRight => (2, 0),
            Self::CenterLeft => (0, 1),
            Self::Center => (1, 1),
            Self::CenterRight => (2, 1),
            Self::BottomLeft => (0, 2),
            Self::BottomCenter => (1, 2),
            Self::BottomRight => (2, 2),
        };
        [
            match x {
                0 => 4.,
                1 => (width - w) * 0.5,
                _ => width - w - 4.,
            },
            match y {
                0 => 4.,
                1 => (height - h) * 0.5,
                _ => height - h - 4.,
            },
        ]
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivationSettings {
    pub mode: Mode,
    pub key: String,
    pub distance: u16,
}
impl Default for ActivationSettings {
    fn default() -> Self {
        Self {
            mode: Mode::Inherit,
            key: String::new(),
            distance: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub enabled: bool,
    pub muted: bool,
    pub volume: f32,
    pub microphone_gain: f32,
    pub input_device: String,
    pub output_device: String,
    /// Zero follows the server's default; other values select its nearest allowed distance.
    pub distance: u16,
    pub deafened: bool,
    pub mode: Mode,
    pub threshold_db: f32,
    pub show_hud: bool,
    pub show_speakers: bool,
    pub show_player_icons: bool,
    pub show_idle: bool,
    pub hud_corner: u8,
    pub hud_scale: f32,
    pub mono_sources: bool,
    pub player_volumes: HashMap<u128, f32>,
    pub player_muted: HashSet<u128>,
    pub line_muted: HashSet<u128>,
    pub line_volumes: HashMap<u128, f32>,
    pub line_overlays: HashMap<u128, OverlayMode>,
    pub activations: HashMap<u128, ActivationSettings>,
    pub noise_suppression: bool,
    pub stereo_capture: bool,
    pub disable_input_device: bool,
    pub sound_occlusion: bool,
    pub directional_sources: bool,
    pub hrtf: bool,
    pub compressor_limiter: bool,
    pub compressor_threshold: f32,
    pub limiter_threshold: f32,
    pub panning: bool,
    pub camera_sound_listener: bool,
    pub exponential_volume_slider: bool,
    pub exponential_distance_gain: bool,
    pub adaptive_jitter_buffer: bool,
    pub visualize_distance: bool,
    pub visualize_on_join: bool,
    pub directional_angle: f32,
    pub jitter_packet_delay: u8,
    pub playback_buffers: u8,
    pub overlap: SourceOverlap,
    pub icon_visibility: IconVisibility,
    pub static_icons: bool,
    pub hud_position: Anchor,
    pub overlay_position: Anchor,
    pub overlay_style: OverlayStyle,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "en_us".into(),
            enabled: true,
            muted: false,
            volume: 1.0,
            microphone_gain: 1.0,
            input_device: String::new(),
            output_device: String::new(),
            distance: 0,
            deafened: false,
            mode: Mode::PushToTalk,
            threshold_db: -30.0,
            show_hud: true,
            show_speakers: true,
            show_player_icons: true,
            show_idle: false,
            hud_corner: 3,
            hud_scale: 1.0,
            mono_sources: false,
            player_volumes: HashMap::new(),
            player_muted: HashSet::new(),
            line_muted: HashSet::new(),
            line_volumes: HashMap::new(),
            line_overlays: HashMap::new(),
            activations: HashMap::new(),
            noise_suppression: false,
            stereo_capture: false,
            disable_input_device: false,
            sound_occlusion: false,
            directional_sources: false,
            hrtf: false,
            compressor_limiter: true,
            compressor_threshold: -10.,
            limiter_threshold: -6.,
            panning: true,
            camera_sound_listener: true,
            exponential_volume_slider: true,
            exponential_distance_gain: true,
            adaptive_jitter_buffer: false,
            visualize_distance: true,
            visualize_on_join: false,
            directional_angle: 145.,
            jitter_packet_delay: 3,
            playback_buffers: 5,
            overlap: SourceOverlap::default(),
            icon_visibility: IconVisibility::default(),
            static_icons: true,
            hud_position: Anchor::default(),
            overlay_position: Anchor::TopLeft,
            overlay_style: OverlayStyle::default(),
        }
    }
}
impl Settings {
    /// Each slider is an independent multiplier. Apply the optional response
    /// curve to the product without capping one slider at another's level.
    pub fn source_volume(&self, player: u128, line: u128, line_default: f32) -> f32 {
        if self.player_muted.contains(&player) || self.line_muted.contains(&line) {
            return 0.;
        }
        let player = self.player_volumes.get(&player).copied().unwrap_or(1.);
        let line = self
            .line_volumes
            .get(&line)
            .copied()
            .unwrap_or(line_default);
        let gain = self.volume * player * line;
        if self.exponential_volume_slider {
            gain * gain
        } else {
            gain
        }
    }
    pub fn normalize(&mut self) {
        self.volume = if self.volume.is_finite() {
            self.volume.clamp(0.0, 2.0)
        } else {
            1.0
        };
        self.microphone_gain = if self.microphone_gain.is_finite() {
            self.microphone_gain.clamp(0.0, 4.0)
        } else {
            1.0
        };
        self.threshold_db = if self.threshold_db.is_finite() {
            self.threshold_db.clamp(-60.0, 0.0)
        } else {
            -35.0
        };
        self.hud_scale = if self.hud_scale.is_finite() {
            self.hud_scale.clamp(0.75, 2.0)
        } else {
            1.0
        };
        self.hud_corner %= 4;
        if !self.compressor_threshold.is_finite() {
            self.compressor_threshold = -10.;
        }
        if !self.limiter_threshold.is_finite() {
            self.limiter_threshold = -6.;
        }
        self.compressor_threshold = self.compressor_threshold.clamp(-60., 0.);
        self.limiter_threshold = self.limiter_threshold.clamp(-30., 0.);
        if !self.directional_angle.is_finite() {
            self.directional_angle = 145.;
        }
        self.directional_angle = self.directional_angle.clamp(0., 360.);
        self.jitter_packet_delay = self.jitter_packet_delay.clamp(1, 10);
        self.playback_buffers = self.playback_buffers.clamp(2, 12);
        self.line_volumes.retain(|_, v| v.is_finite());
        for value in self.line_volumes.values_mut() {
            *value = value.clamp(0., 2.);
        }
        self.player_volumes.retain(|_, value| value.is_finite());
        for value in self.player_volumes.values_mut() {
            *value = value.clamp(0.0, 2.0);
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Connecting,
    Connected,
    ListenOnly,
    TimedOut,
    Error,
}
pub enum Event {
    Tcp(Vec<u8>),
    Status(Status),
    Error(String),
    ActionBar(String),
    Visualize {
        radius: i32,
        color: u32,
        position: Option<[f64; 3]>,
    },
}
#[derive(Clone, Default)]
pub struct SpeakerView {
    pub source: u128,
    pub line: u128,
    pub player: u128,
    pub name: String,
    pub speaking: bool,
    pub position: Option<[f64; 3]>,
    pub audible: bool,
    pub static_source: bool,
    pub direct: bool,
    pub icon_visible: bool,
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub level: f32,
    pub transmitting: bool,
    pub transmitting_activations: Vec<u128>,
    pub microphone_open: bool,
    pub output_open: bool,
    pub output_level: f32,
    pub speakers: Vec<SpeakerView>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub distances: Vec<u16>,
    pub distance: u16,
    pub sample_rate: u32,
    pub latency_ms: u64,
    pub sent: u64,
    pub received: u64,
    pub activations: Arc<Vec<protocol::Activation>>,
    pub lines: Arc<Vec<protocol::SourceLine>>,
    pub players: Arc<Vec<protocol::VoicePlayer>>,
    pub languages: Arc<HashMap<String, HashMap<String, String>>>,
    pub permissions: Arc<HashMap<String, bool>>,
    pub hidden_icons: Arc<Vec<String>>,
    pub icon_offset: [f64; 3],
}
enum Command {
    Control(Vec<u8>),
    Settings(Box<Settings>),
    Stop,
    Preview(bool),
    Devices,
}

pub struct Client {
    tx: SyncSender<Command>,
    rx: Receiver<Event>,
    pub talking: Arc<AtomicBool>,
    pub active: Arc<AtomicBool>,
    pub metering: Arc<AtomicBool>,
    pub keys: Arc<Mutex<HashSet<u128>>>,
    scene: Arc<Mutex<Scene>>,
    snapshot: Arc<Mutex<Snapshot>>,
}
impl Client {
    pub fn start(
        server_host: String,
        key: KeyFactory,
        activation_id: ActivationId,
        settings: Settings,
        devices: bool,
    ) -> Result<Self> {
        let (tx, commands) = mpsc::sync_channel(128);
        let (events, rx) = mpsc::sync_channel(128);
        let talking = Arc::new(AtomicBool::new(false));
        let scene = Arc::new(Mutex::new(Scene::default()));
        let active = Arc::new(AtomicBool::new(false));
        let worker_active = active.clone();
        let metering = Arc::new(AtomicBool::new(false));
        let worker_metering = metering.clone();
        let keys = Arc::new(Mutex::new(HashSet::new()));
        let worker_keys = keys.clone();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let worker_snapshot = snapshot.clone();
        let worker_talking = talking.clone();
        let worker_scene = scene.clone();
        thread::Builder::new()
            .name("Plasmo Voice".into())
            .spawn(move || {
                if let Err(error) = Worker::new(
                    server_host,
                    key,
                    activation_id,
                    settings,
                    devices,
                    commands,
                    events.clone(),
                    worker_talking,
                    worker_scene,
                    worker_active,
                    worker_snapshot,
                    worker_metering,
                    worker_keys,
                )
                .run()
                {
                    let _ = events.try_send(Event::Error(error));
                    let _ = events.try_send(Event::Status(Status::Error));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            tx,
            rx,
            talking,
            scene,
            active,
            snapshot,
            metering,
            keys,
        })
    }
    pub fn control(&self, bytes: &[u8]) -> bool {
        bytes.len() <= crate::wire::MAX_CONTROL
            && self.tx.try_send(Command::Control(bytes.to_vec())).is_ok()
    }
    pub fn settings(&self, settings: Settings) -> bool {
        self.tx
            .try_send(Command::Settings(Box::new(settings)))
            .is_ok()
    }
    pub fn scene(&self, scene: Scene) {
        if let Ok(mut current) = self.scene.try_lock() {
            *current = scene;
        }
    }
    pub fn events(&self) -> impl Iterator<Item = Event> + '_ {
        self.rx.try_iter().take(128)
    }
    pub fn preview(&self, enabled: bool) {
        let _ = self.tx.try_send(Command::Preview(enabled));
    }
    pub fn refresh_devices(&self) {
        let _ = self.tx.try_send(Command::Devices);
    }
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.snapshot.try_lock().ok().map(|s| s.clone())
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.talking.store(false, Ordering::Relaxed);
        self.active.store(false, Ordering::Relaxed);
        self.metering.store(false, Ordering::Relaxed);
        let _ = self.tx.try_send(Command::Stop);
    }
}

struct Speaker {
    info: Source,
    decoder: Decoder,
    jitter: Jitter,
    pcm: VecDeque<i16>,
    distance: f32,
    last: Instant,
    filter: [f32; 2],
}
struct Worker {
    host: String,
    key_factory: KeyFactory,
    key: Option<Box<dyn KeyPair>>,
    activation_id: ActivationId,
    settings: Settings,
    devices: bool,
    commands: Receiver<Command>,
    events: SyncSender<Event>,
    talking: Arc<AtomicBool>,
    scene: Arc<Mutex<Scene>>,
    socket: Option<UdpSocket>,
    secret: u128,
    destination: String,
    port: u16,
    config: Option<protocol::Config>,
    cipher: Option<AudioCipher>,
    opus: Option<Arc<Opus>>,
    encoders: HashMap<u128, (u32, Option<protocol::Codec>, Encoder)>,
    activation: Option<(u128, u16)>,
    capture: Option<Capture>,
    output: Option<Output>,
    speakers: HashMap<u128, Speaker>,
    requested: HashMap<u128, Instant>,
    sequence: u64,
    was_talking: bool,
    last_ping: Instant,
    last_udp: Instant,
    connected_at: Instant,
    playout: Instant,
    status: Status,
    active: Arc<AtomicBool>,
    snapshot: Arc<Mutex<Snapshot>>,
    view: Snapshot,
    preview: bool,
    preview_pcm: VecDeque<f32>,
    preview_resampler: Resampler,
    capture_rate: u32,
    capture_retry: Instant,
    activations: Activations,
    metering: Arc<AtomicBool>,
    keys: Arc<Mutex<HashSet<u128>>>,
    dynamics: crate::audio::Dynamics,
    denoiser: Option<Denoiser>,
    hrtf: Option<crate::hrtf::Hrtf>,
    encode_resampler: Resampler,
    encode_rate: u32,
    encode_pending: VecDeque<i16>,
    registry_dirty: bool,
    last_frame: Instant,
}
impl Worker {
    #[allow(clippy::too_many_arguments)]
    fn new(
        host: String,
        key_factory: KeyFactory,
        activation_id: ActivationId,
        mut settings: Settings,
        devices: bool,
        commands: Receiver<Command>,
        events: SyncSender<Event>,
        talking: Arc<AtomicBool>,
        scene: Arc<Mutex<Scene>>,
        active: Arc<AtomicBool>,
        snapshot: Arc<Mutex<Snapshot>>,
        metering: Arc<AtomicBool>,
        keys: Arc<Mutex<HashSet<u128>>>,
    ) -> Self {
        settings.normalize();
        let now = Instant::now();
        Self {
            host,
            key_factory,
            key: None,
            activation_id,
            settings,
            devices,
            commands,
            events,
            talking,
            scene,
            socket: None,
            secret: 0,
            destination: String::new(),
            port: 0,
            config: None,
            cipher: None,
            opus: None,
            encoders: HashMap::new(),
            activation: None,
            capture: None,
            output: None,
            speakers: HashMap::new(),
            requested: HashMap::new(),
            sequence: 0,
            was_talking: false,
            last_ping: now - Duration::from_secs(1),
            last_udp: now,
            connected_at: now,
            playout: now,
            status: Status::Waiting,
            active,
            snapshot,
            view: Snapshot::default(),
            preview: false,
            preview_pcm: VecDeque::new(),
            preview_resampler: Resampler::new(48000, 48000, 1),
            capture_rate: 48000,
            capture_retry: now,
            activations: Activations::default(),
            metering,
            keys,
            dynamics: Default::default(),
            denoiser: None,
            hrtf: None,
            encode_resampler: Resampler::new(48000, 48000, 1),
            encode_rate: 48000,
            encode_pending: VecDeque::new(),
            registry_dirty: true,
            last_frame: now - Duration::from_secs(1),
        }
    }
    fn emit(&self, event: Event) -> Result<()> {
        self.events
            .try_send(event)
            .map_err(|_| "game stopped receiving voice events".into())
    }
    fn status(&mut self, status: Status) -> Result<()> {
        if self.status != status {
            self.status = status;
            self.emit(Event::Status(status))?;
        }
        Ok(())
    }
    fn tcp(&self, bytes: Vec<u8>) -> Result<()> {
        self.emit(Event::Tcp(bytes))
    }
    fn run(mut self) -> Result<()> {
        loop {
            for _ in 0..128 {
                match self.commands.try_recv() {
                    Ok(Command::Stop) | Err(TryRecvError::Disconnected) => {
                        self.end_talk()?;
                        return Ok(());
                    }
                    Err(TryRecvError::Empty) => break,
                    Ok(Command::Devices) => {
                        self.view.inputs = crate::audio::devices(true);
                        self.view.outputs = crate::audio::devices(false);
                    }
                    Ok(Command::Preview(preview)) => {
                        self.preview = preview;
                        self.preview_pcm.clear();
                        if preview {
                            self.open_devices();
                        }
                    }
                    Ok(Command::Settings(settings)) => {
                        let mut settings = *settings;
                        settings.normalize();
                        let state_changed = settings.enabled != self.settings.enabled
                            || settings.muted != self.settings.muted
                            || settings.deafened != self.settings.deafened;
                        let distance_changed = settings.distance != self.settings.distance
                            || settings.activations.iter().any(|(id, v)| {
                                self.settings.activations.get(id).map_or(0, |v| v.distance)
                                    != v.distance
                            });
                        let reset = settings.input_device != self.settings.input_device
                            || settings.output_device != self.settings.output_device
                            || settings.stereo_capture != self.settings.stereo_capture
                            || settings.disable_input_device != self.settings.disable_input_device
                            || settings.playback_buffers != self.settings.playback_buffers;
                        let language_changed = settings.language != self.settings.language;
                        self.settings = settings;
                        if language_changed && self.config.is_some() {
                            self.tcp(protocol::language_request(&self.settings.language)?)?;
                        }
                        for speaker in self.speakers.values_mut() {
                            speaker.jitter.configure(
                                self.settings.jitter_packet_delay,
                                self.settings.adaptive_jitter_buffer,
                            );
                        }
                        if state_changed {
                            self.tcp(protocol::player_state(
                                !self.settings.enabled || self.settings.deafened,
                                self.settings.muted || self.settings.deafened,
                            ))?;
                        }
                        if reset {
                            self.capture = None;
                            self.output = None;
                            self.capture_retry = Instant::now();
                            self.open_devices();
                        }
                        self.choose_activation();
                        if distance_changed {
                            self.publish_distances()?;
                        }
                    }
                    Ok(Command::Control(data)) => self.control(&data)?,
                }
            }
            self.network()?;
            self.record()?;
            let now = Instant::now();
            if now >= self.playout {
                self.play(now);
                self.playout += Duration::from_millis(20);
                if now.duration_since(self.playout.min(now)) > Duration::from_millis(100) {
                    self.playout = now + Duration::from_millis(20);
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
    fn control(&mut self, data: &[u8]) -> Result<()> {
        let packet = match protocol::read_control(data) {
            Ok(packet) => packet,
            Err(e) => {
                log::warn!("Plasmo Voice ignored malformed control packet: {e}");
                return Ok(());
            }
        };
        if matches!(
            &packet,
            Control::Config(_)
                | Control::Permissions(_)
                | Control::ActivationRegister(_)
                | Control::ActivationUnregister(_)
                | Control::LineRegister(_)
                | Control::LineUnregister(_)
                | Control::LinePlayers { .. }
                | Control::LinePlayerAdd { .. }
                | Control::LinePlayerRemove { .. }
        ) {
            self.registry_dirty = true;
        }
        match packet {
            Control::PlayerInfoRequest => {
                if self.key.is_none() {
                    self.key = Some((self.key_factory)()?);
                }
                self.tcp(protocol::player_info(
                    "26.3",
                    &self.key.as_ref().unwrap().public_der(),
                    !self.settings.enabled || self.settings.deafened,
                    self.settings.muted || self.settings.deafened,
                )?)?;
            }
            Control::Connection { secret, host, port } => {
                self.end_talk()?;
                self.capture = None;
                self.output = None;
                self.speakers.clear();
                self.requested.clear();
                self.config = None;
                self.cipher = None;
                self.encoders.clear();
                self.secret = secret;
                self.destination = if host.is_empty() || host == "0.0.0.0" || host == "::" {
                    self.host.clone()
                } else {
                    host
                };
                self.port = port;
                let addresses = (self.destination.as_str(), port)
                    .to_socket_addrs()
                    .map_err(|e| e.to_string())?;
                let mut connected = None;
                for address in addresses.take(16) {
                    let bind = if address.is_ipv4() {
                        "0.0.0.0:0"
                    } else {
                        "[::]:0"
                    };
                    if let Ok(socket) = UdpSocket::bind(bind)
                        && socket.connect(address).is_ok()
                    {
                        connected = Some(socket);
                        break;
                    }
                }
                let socket = connected.ok_or("cannot connect to the voice endpoint")?;
                socket.set_nonblocking(true).map_err(|e| e.to_string())?;
                self.socket = Some(socket);
                self.connected_at = Instant::now();
                self.last_udp = self.connected_at;
                self.last_ping = self.connected_at - Duration::from_secs(1);
                self.status(Status::Connecting)?;
            }
            Control::Config(mut config) => {
                let cipher = if let Some((algorithm, data)) = &config.encryption {
                    if algorithm != "AES/CBC/PKCS5Padding" {
                        return Err("unsupported voice encryption".into());
                    }
                    let mut key = self
                        .key
                        .as_ref()
                        .ok_or("server configuration arrived before key exchange")?
                        .decrypt(data)?;
                    let cipher = AudioCipher::new(&key);
                    key.fill(0);
                    Some(cipher?)
                } else {
                    None
                };
                if config
                    .encoder
                    .as_ref()
                    .is_some_and(|c| !c.name.eq_ignore_ascii_case("opus"))
                {
                    return Err("unsupported capture codec".into());
                }
                if self.opus.is_none() {
                    self.opus = Some(Opus::load()?);
                }
                self.encoders.clear();
                self.cipher = cipher;
                for activation in &mut config.activations {
                    activation.id = (self.activation_id)(&activation.name, "activation");
                }
                for line in &mut config.lines {
                    line.id = (self.activation_id)(&line.name, "line");
                }
                self.view.hidden_icons = Arc::new(config.hidden_icons.clone());
                self.view.icon_offset = config.icon_offset;
                self.config = Some(config);
                self.choose_activation();
                self.tcp(protocol::language_request(&self.settings.language)?)?;
                self.publish_distances()?;
                self.open_devices();
                self.status(Status::Connected)?;
            }
            Control::Source(info) => self.install_source(info)?,
            Control::SourceEnd { source, .. } => {
                // Keep the short queued tail: the reliable TCP end may overtake UDP.
                if let Some(speaker) = self.speakers.get_mut(&source) {
                    speaker.last = Instant::now();
                }
            }
            Control::Permissions(values) => {
                if let Some(config) = &mut self.config {
                    config.permissions = values;
                }
                self.choose_activation();
            }
            Control::Language { name, values } => {
                Arc::make_mut(&mut self.view.languages).insert(name, values);
            }
            Control::Players(players) => {
                self.view.players = Arc::new(players);
            }
            Control::PlayerUpdate(info) => {
                Arc::make_mut(&mut self.view.players).retain(|p| p.id != info.id);
                Arc::make_mut(&mut self.view.players).push(info.clone());
                for speaker in self
                    .speakers
                    .values_mut()
                    .filter(|s| s.info.player == info.id)
                {
                    speaker.info.muted =
                        info.server_muted || info.voice_disabled || info.microphone_muted;
                    speaker.info.player_info = Some(info.clone());
                }
            }
            Control::PlayerDisconnect(id) => {
                Arc::make_mut(&mut self.view.players).retain(|p| p.id != id);
                self.speakers.retain(|_, s| s.info.player != id);
            }
            Control::ActivationRegister(mut activation) => {
                activation.id = (self.activation_id)(&activation.name, "activation");
                if let Some(config) = &mut self.config {
                    config.activations.retain(|a| a.id != activation.id);
                    config.activations.push(activation);
                }
                self.choose_activation();
            }
            Control::ActivationUnregister(id) => {
                self.encoders.remove(&id);
                if let Some(config) = &mut self.config {
                    config.activations.retain(|a| a.id != id);
                }
                self.choose_activation();
            }
            Control::LineRegister(mut line) => {
                line.id = (self.activation_id)(&line.name, "line");
                if let Some(config) = &mut self.config {
                    config.lines.retain(|l| l.id != line.id);
                    config.lines.push(line);
                }
            }
            Control::LineUnregister(id) => {
                if let Some(config) = &mut self.config {
                    config.lines.retain(|l| l.id != id);
                }
                self.speakers.retain(|_, s| s.info.line != id);
            }
            Control::LinePlayers { line, players } => {
                if let Some(info) = self
                    .config
                    .as_mut()
                    .and_then(|c| c.lines.iter_mut().find(|l| l.id == line))
                {
                    info.players = Some(players);
                }
            }
            Control::LinePlayerAdd { line, player } => {
                if let Some(info) = self
                    .config
                    .as_mut()
                    .and_then(|c| c.lines.iter_mut().find(|l| l.id == line))
                {
                    let players = info.players.get_or_insert_default();
                    players.retain(|p| p.id != player.id);
                    if players.len() < 1024 {
                        players.push(player);
                    }
                }
            }
            Control::LinePlayerRemove { line, player } => {
                if let Some(info) = self
                    .config
                    .as_mut()
                    .and_then(|c| c.lines.iter_mut().find(|l| l.id == line))
                    && let Some(players) = &mut info.players
                {
                    players.retain(|p| p.id != player);
                }
            }
            Control::SelfSource { source, .. } => self.install_source(source)?,
            Control::DistanceVisualize {
                radius,
                color,
                position,
            } => self.emit(Event::Visualize {
                radius,
                color,
                position,
            })?,
            Control::ActionBar(message) => self.emit(Event::ActionBar(message))?,
            Control::Unknown(_) => {}
        }
        Ok(())
    }
    fn install_source(&mut self, info: Source) -> Result<()> {
        if self.speakers.len() >= 64 && !self.speakers.contains_key(&info.id) {
            return Ok(());
        }
        if let Some(speaker) = self.speakers.get_mut(&info.id)
            && speaker.info.state == info.state
            && speaker.info.stereo == info.stereo
        {
            speaker.info = info;
            return Ok(());
        }
        let Some(opus) = self.opus.as_ref() else {
            return Ok(());
        };
        let decoder = opus.decoder(info.stereo)?;
        self.requested.remove(&info.id);
        let mut jitter = Jitter::default();
        jitter.configure(
            self.settings.jitter_packet_delay,
            self.settings.adaptive_jitter_buffer,
        );
        self.speakers.insert(
            info.id,
            Speaker {
                info,
                decoder,
                jitter,
                pcm: VecDeque::new(),
                distance: 16.0,
                last: Instant::now() - Duration::from_secs(1),
                filter: [0.; 2],
            },
        );

        Ok(())
    }
    fn choose_activation(&mut self) {
        self.activation = self
            .config
            .as_ref()
            .and_then(|c| {
                c.activations
                    .iter()
                    .find(|a| a.proximity && a.name == "proximity")
                    .or_else(|| c.activations.iter().find(|a| a.proximity))
            })
            .and_then(|a| {
                let config = self.config.as_ref().unwrap();
                if config.permissions.get(&format!("pv.activation.{}", a.name)) == Some(&false) {
                    return None;
                }
                let distance = if self.settings.distance == 0 {
                    a.default_distance
                } else {
                    if a.dynamic_distance {
                        self.settings
                            .distance
                            .clamp(1, a.distances.last().copied().unwrap_or(32767))
                    } else {
                        a.distances
                            .iter()
                            .min_by_key(|d| d.abs_diff(self.settings.distance))
                            .copied()
                            .unwrap_or(a.default_distance)
                    }
                };
                Some(((self.activation_id)(&a.name, "activation"), distance))
            });
    }
    fn publish_distances(&self) -> Result<()> {
        if let Some(config) = &self.config {
            let values: Vec<_> = config
                .activations
                .iter()
                .map(|a| (a.id, crate::activations::distance(a, &self.settings)))
                .collect();
            self.tcp(protocol::activation_distances(&values))?;
        }
        Ok(())
    }
    fn open_devices(&mut self) {
        if !self.devices {
            return;
        }
        if self.output.is_none() {
            match Output::buffered(&self.settings.output_device, self.settings.playback_buffers) {
                Ok(d) => self.output = Some(d),
                Err(e) => {
                    let _ = self.emit(Event::Error(e));
                }
            }
        }
        // Capture opens only after a user presses PTT, in record().
    }
    fn network(&mut self) -> Result<()> {
        let Some(socket) = self.socket.as_ref() else {
            return Ok(());
        };
        let now = Instant::now();
        if now.duration_since(self.last_ping) >= Duration::from_millis(500) {
            let endpoint = self
                .config
                .is_none()
                .then_some((self.destination.as_str(), self.port));
            let bytes = protocol::ping(self.secret, timestamp(), endpoint)?;
            let _ = socket.send(&bytes);
            self.last_ping = now;
        }
        for _ in 0..256 {
            let mut bytes = [0; 8193];
            let n = match socket.recv(&mut bytes) {
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => break,
                Err(e) => return Err(e.to_string()),
            };
            let Ok(packet) = protocol::read_datagram(&bytes[..n], self.secret) else {
                continue;
            };
            self.last_udp = now;
            self.view.received += 1;
            if let Datagram::Ping { timestamp: sent } = packet {
                self.view.latency_ms = timestamp().saturating_sub(sent).min(60_000);
                continue;
            }
            if let Datagram::Audio {
                sequence,
                data,
                source,
                state,
                distance,
            } = packet
            {
                if self
                    .speakers
                    .get(&source)
                    .is_none_or(|speaker| speaker.info.state != state)
                {
                    if self.requested.len() < 256
                        && self
                            .requested
                            .get(&source)
                            .is_none_or(|t| now.duration_since(*t) > Duration::from_secs(1))
                    {
                        self.requested.insert(source, now);
                        self.tcp(protocol::source_request(source))?;
                    }
                    continue;
                }
                let speaker = self.speakers.get_mut(&source).unwrap();
                if speaker.info.muted
                    || !self.settings.enabled
                    || (distance == 0
                        && !matches!(speaker.info.kind, protocol::SourceKind::Direct { .. }))
                {
                    continue;
                }
                let data = match &self.cipher {
                    Some(cipher) => match cipher.decrypt(&data) {
                        Ok(d) => d,
                        Err(_) => continue,
                    },
                    None => data,
                };
                speaker.distance = f32::from(distance.min(32767));
                speaker.last = now;
                speaker.jitter.push(sequence, data, now);
            }
        }
        self.requested
            .retain(|_, t| now.duration_since(*t) < Duration::from_secs(5));
        if now.duration_since(self.last_udp) > Duration::from_secs(15) {
            self.end_talk()?;
            self.capture = None;
            self.output = None;
            self.socket = None;
            self.speakers.clear();
            self.status(Status::TimedOut)?;
        }
        Ok(())
    }
    fn record(&mut self) -> Result<()> {
        let held = self.talking.load(Ordering::Relaxed);
        let keys = self.keys.lock().map(|v| v.clone()).unwrap_or_default();
        let allowed = self.active.load(Ordering::Relaxed)
            && !self.settings.disable_input_device
            && !self.settings.muted
            && !self.settings.deafened
            && self.settings.enabled
            && self.socket.is_some()
            && self.config.is_some();
        let wants = self.config.as_ref().is_some_and(|c| {
            Activations::wants_capture(&c.activations, &self.settings, &keys, held)
        });
        let sampling = !self.settings.disable_input_device
            && (self.preview || self.metering.load(Ordering::Relaxed) || (allowed && wants));
        let encode_rate = self.config.as_ref().map_or(48000, |c| c.sample_rate);
        let rate = if self.settings.noise_suppression {
            48000
        } else {
            encode_rate
        };
        if rate != self.capture_rate || encode_rate != self.encode_rate {
            self.capture = None;
            self.capture_rate = rate;
            self.encode_rate = encode_rate;
            self.preview_resampler = Resampler::new(rate, 48000, 1);
            self.encode_resampler = Resampler::new(rate, encode_rate, 1);
            self.encode_pending.clear();
            self.denoiser = None;
        }
        if sampling
            && self.devices
            && self.capture.is_none()
            && Instant::now() >= self.capture_retry
        {
            match Capture::open_options(
                &self.settings.input_device,
                rate,
                self.settings.stereo_capture,
            ) {
                Ok(capture) => {
                    self.capture = Some(capture);
                    if self.config.is_some() {
                        self.status(Status::Connected)?;
                    }
                }
                Err(e) => {
                    self.view.level = 0.;
                    self.emit(Event::Error(e))?;
                    self.status(Status::ListenOnly)?;
                    self.talking.store(false, Ordering::Relaxed);
                    self.capture_retry = Instant::now() + Duration::from_secs(3);
                    return Ok(());
                }
            }
        }
        if !sampling || !allowed {
            self.end_talk()?;
        }
        if !sampling {
            self.view.level *= 0.8;
        }
        let Some(capture) = self.capture.as_mut() else {
            return Ok(());
        };
        if capture.failed() {
            self.view.level = 0.;
            self.capture = None;
            self.capture_retry = Instant::now() + Duration::from_secs(3);
            self.end_talk()?;
            self.status(Status::ListenOnly)?;
            return Ok(());
        }
        capture.set_transmitting(sampling);
        let frames = capture.frames(self.settings.microphone_gain);
        for mut pcm in frames {
            if self.settings.noise_suppression {
                if self.denoiser.is_none() {
                    self.denoiser = Some(Denoiser::load()?);
                }
                self.denoiser.as_mut().unwrap().process(&mut pcm)?;
            }
            if self.settings.compressor_limiter {
                self.dynamics.process(
                    &mut pcm,
                    rate,
                    self.settings.compressor_threshold,
                    self.settings.limiter_threshold,
                );
            }
            self.view.level = activation::level(&pcm);
            if self.preview {
                let floats: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.).collect();
                self.preview_pcm
                    .extend(self.preview_resampler.push(&floats));
                if self.preview_pcm.len() > 4800 {
                    self.preview_pcm.clear();
                }
            }
            let Some(config) = &self.config else {
                continue;
            };
            let budget = config.mtu.saturating_sub(128).clamp(64, 1275);
            let plan = self.activations.update(
                &config.activations,
                &self.settings,
                &config.permissions,
                &keys,
                held,
                allowed,
                self.view.level,
                Instant::now(),
            );
            for target in &plan.ended {
                self.tcp(protocol::player_end(
                    self.sequence,
                    target.id,
                    target.distance,
                ))?;
            }
            if plan.targets.is_empty() {
                self.was_talking = false;
                self.view.transmitting_activations.clear();
                self.encode_pending.clear();
                continue;
            }
            let frames = if rate == encode_rate {
                vec![pcm]
            } else {
                let input: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.).collect();
                self.encode_pending.extend(
                    self.encode_resampler
                        .push(&input)
                        .into_iter()
                        .map(|v| (v * 32768.).clamp(-32768., 32767.) as i16),
                );
                let samples = encode_rate as usize / 50;
                let mut frames = Vec::new();
                while self.encode_pending.len() >= samples {
                    frames.push(self.encode_pending.drain(..samples).collect());
                }
                frames
            };
            for pcm in frames {
                let mut sent = false;
                for target in &plan.targets {
                    let info = self
                        .config
                        .as_ref()
                        .and_then(|c| {
                            c.activations
                                .iter()
                                .find(|a| a.id == target.id)
                                .and_then(|a| a.encoder.as_ref())
                                .or(c.encoder.as_ref())
                        })
                        .cloned();
                    if self
                        .encoders
                        .get(&target.id)
                        .is_none_or(|(rate, config, _)| *rate != encode_rate || config != &info)
                    {
                        let encoder = self
                            .opus
                            .as_ref()
                            .unwrap()
                            .configured_encoder(encode_rate, info.as_ref())?;
                        self.encoders
                            .insert(target.id, (encode_rate, info, encoder));
                    }
                    let opus = self
                        .encoders
                        .get_mut(&target.id)
                        .unwrap()
                        .2
                        .encode(&pcm, budget)?;
                    let data = match &self.cipher {
                        Some(cipher) => cipher.encrypt(&opus)?,
                        None => opus,
                    };
                    let packet = protocol::player_audio(
                        self.secret,
                        timestamp(),
                        self.sequence,
                        &data,
                        target.id,
                        target.distance,
                    );
                    if self.socket.as_ref().unwrap().send(&packet).is_ok() {
                        self.view.sent += 1;
                        sent = true;
                    }
                }
                self.sequence = self.sequence.wrapping_add(1);
                if sent {
                    self.was_talking = true;
                    self.view.transmitting_activations =
                        plan.targets.iter().map(|t| t.id).collect();
                    self.last_frame = Instant::now();
                }
            }
        }
        Ok(())
    }
    fn end_talk(&mut self) -> Result<()> {
        for target in self.activations.stop() {
            self.tcp(protocol::player_end(
                self.sequence,
                target.id,
                target.distance,
            ))?;
        }
        self.was_talking = false;
        self.view.transmitting_activations.clear();
        self.encode_pending.clear();
        Ok(())
    }
    fn play(&mut self, now: Instant) {
        if self.settings.hrtf && self.hrtf.is_none() {
            match crate::hrtf::Hrtf::new(self.settings.playback_buffers) {
                Ok(renderer) => self.hrtf = Some(renderer),
                Err(e) => {
                    let _ = self.emit(Event::Error(e));
                    self.settings.hrtf = false;
                }
            }
        } else if !self.settings.hrtf {
            self.hrtf = None;
        }
        let scene = self.scene.lock().map(|s| s.clone()).unwrap_or_default();
        let mut mixed = vec![0.0f32; 960 * 2];
        self.speakers
            .retain(|_, s| now.duration_since(s.last) < Duration::from_secs(30));
        let direct: HashSet<_> = self
            .speakers
            .values()
            .filter(|s| {
                s.info.player != 0
                    && matches!(s.info.kind, protocol::SourceKind::Direct { .. })
                    && now.duration_since(s.last) < Duration::from_millis(250)
            })
            .map(|s| s.info.player)
            .collect();
        let proximity: HashSet<_> = self
            .speakers
            .values()
            .filter(|s| {
                s.info.player != 0
                    && matches!(s.info.kind, protocol::SourceKind::Player)
                    && now.duration_since(s.last) < Duration::from_millis(250)
            })
            .map(|s| s.info.player)
            .collect();
        for speaker in self.speakers.values_mut() {
            let decoded = match speaker.jitter.pop(now) {
                Playout::Packet(bytes) => speaker.decoder.decode(Some(&bytes)),
                Playout::Lost => speaker.decoder.decode(None),
                Playout::Idle => Ok(Vec::new()),
            };
            if let Ok(pcm) = decoded {
                speaker.pcm.extend(pcm);
            }
            let channels = speaker.decoder.channels();
            if speaker.pcm.len() > 5760 * channels {
                speaker.pcm.clear();
            }
            let default = self
                .config
                .as_ref()
                .and_then(|c| c.lines.iter().find(|l| l.id == speaker.info.line))
                .map_or(1., |l| l.default_volume);
            let gain = self
                .settings
                .source_volume(speaker.info.player, speaker.info.line, default);
            let overlap = match self.settings.overlap {
                SourceOverlap::MuteProximity => {
                    matches!(speaker.info.kind, protocol::SourceKind::Player)
                        && direct.contains(&speaker.info.player)
                }
                SourceOverlap::MuteDirect => {
                    matches!(speaker.info.kind, protocol::SourceKind::Direct { .. })
                        && proximity.contains(&speaker.info.player)
                }
                SourceOverlap::Off => false,
            };
            let occlusion = if self.settings.sound_occlusion
                && !matches!(speaker.info.kind, protocol::SourceKind::Direct { .. })
            {
                scene.occlusion.get(&speaker.info.id).copied().unwrap_or(1.)
            } else {
                1.
            };
            let gains = if self.settings.enabled
                && !self.settings.deafened
                && !speaker.info.muted
                && !overlap
            {
                spatial::source_gains(
                    &scene,
                    &speaker.info,
                    speaker.distance,
                    gain * occlusion,
                    self.settings.panning,
                    self.settings.mono_sources,
                    self.settings.exponential_distance_gain,
                    self.settings
                        .directional_sources
                        .then_some(self.settings.directional_angle),
                )
            } else {
                [0.0; 2]
            };
            let binaural = self.hrtf.is_some()
                && self.settings.panning
                && (channels == 1 || self.settings.mono_sources);
            let mut mono = if binaural {
                Vec::with_capacity(960)
            } else {
                Vec::new()
            };
            for frame in mixed.as_chunks_mut::<2>().0 {
                let mut left = f32::from(speaker.pcm.pop_front().unwrap_or(0)) / 32768.0;
                let mut right = if channels == 2 {
                    f32::from(speaker.pcm.pop_front().unwrap_or(0)) / 32768.0
                } else {
                    left
                };
                if self.settings.mono_sources {
                    left = (left + right) * 0.5;
                    right = left;
                }
                let low_pass = if occlusion < 0.99 {
                    0.025 + occlusion * 0.4
                } else {
                    1.
                };
                speaker.filter[0] += low_pass * (left - speaker.filter[0]);
                speaker.filter[1] += low_pass * (right - speaker.filter[1]);
                left = speaker.filter[0];
                right = speaker.filter[1];
                if binaural {
                    mono.push((left * 32768.).clamp(-32768., 32767.) as i16);
                } else {
                    frame[0] += left * gains[0];
                    frame[1] += right * gains[1];
                }
            }
            if binaural {
                let position =
                    spatial::source_position(&scene, &speaker.info).unwrap_or(scene.listener);
                let d: [f32; 3] = std::array::from_fn(|i| (position[i] - scene.listener[i]) as f32);
                let yaw = scene.yaw.to_radians();
                let forward = d[0] * yaw.sin() - d[2] * yaw.cos();
                let pitch = scene.pitch.to_radians();
                let relative = [
                    -d[0] * yaw.cos() - d[2] * yaw.sin(),
                    d[1] * pitch.cos() - forward * pitch.sin(),
                    d[1] * pitch.sin() + forward * pitch.cos(),
                ];
                if let Some(renderer) = &mut self.hrtf {
                    let gain = (gains[0] * gains[0] + gains[1] * gains[1]).sqrt();
                    if let Err(e) = renderer.push(speaker.info.id, &mono, relative, gain) {
                        log::warn!("{e}");
                    }
                }
            }
        }
        if let Some(renderer) = &mut self.hrtf {
            renderer.retain(&self.speakers.keys().copied().collect());
            for (sample, hrtf) in mixed.iter_mut().zip(renderer.render()) {
                *sample += hrtf;
            }
        }
        if self.preview {
            for frame in mixed.as_chunks_mut::<2>().0 {
                let value = self.preview_pcm.pop_front().unwrap_or(0.0) * 0.65;
                frame[0] += value;
                frame[1] += value;
            }
        }
        self.view.output_level =
            (mixed.iter().map(|v| v * v).sum::<f32>() / mixed.len() as f32).sqrt();
        self.view.transmitting =
            self.was_talking && now.duration_since(self.last_frame) < Duration::from_millis(150);
        self.view.microphone_open = self.capture.is_some();
        self.view.output_open = self.output.is_some();
        self.view.speakers = self
            .speakers
            .values()
            .map(|s| SpeakerView {
                source: s.info.id,
                line: s.info.line,
                player: s.info.player,
                name: s.info.name.clone(),
                speaking: now.duration_since(s.last) < Duration::from_millis(250) && !s.info.muted,
                position: spatial::source_position(&scene, &s.info),
                static_source: matches!(
                    s.info.kind,
                    protocol::SourceKind::Static { .. } | protocol::SourceKind::Entity(_)
                ),
                direct: matches!(s.info.kind, protocol::SourceKind::Direct { .. }),
                icon_visible: s.info.icon_visible,
                audible: spatial::source_gains(
                    &scene,
                    &s.info,
                    s.distance,
                    1.,
                    false,
                    false,
                    self.settings.exponential_distance_gain,
                    None,
                )[0] > 0.001,
            })
            .collect();
        self.view.sample_rate = self.config.as_ref().map_or(0, |c| c.sample_rate);
        if self.registry_dirty
            && let Some(config) = &self.config
        {
            self.view.activations = Arc::new(config.activations.clone());
            self.view.lines = Arc::new(config.lines.clone());
            self.view.permissions = Arc::new(config.permissions.clone());
            self.registry_dirty = false;
        }
        self.view.distance = self.activation.map_or(0, |a| a.1);
        self.view.distances = self
            .config
            .as_ref()
            .and_then(|c| c.activations.iter().find(|a| a.proximity))
            .map_or_else(Vec::new, |a| a.distances.clone());
        if let Ok(mut snapshot) = self.snapshot.try_lock() {
            *snapshot = self.view.clone();
        }
        if let Some(output) = self.output.as_mut() {
            if output.failed() {
                self.output = None;
            } else {
                output.write(&mixed);
            }
        }
    }
}
pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
