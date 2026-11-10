// Thin Horizon adapter. Protocol, devices and codec work live on the engine worker.
use horizon_client::mod_api::winit;
use horizon_client::mod_api::{Host, NativeClientHead};
use horizon_plasmo_engine::{
    activation::Mode,
    protocol,
    session::{Client, Event, KeyPair, Settings, Snapshot, Status},
    spatial::Scene,
};
use horizon_protocol::{
    packets::{configuration::CustomPayload, play::Serverbound},
    rsa::RsaPrivateKey,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Mutex, OnceLock, atomic::Ordering},
    time::{Duration, Instant},
};

pub(super) const ID: &str = "plasmo_voice";
pub(super) struct Visualization {
    pub(super) center: [f64; 3],
    pub(super) radius: f32,
    pub(super) color: u32,
    pub(super) at: Instant,
}
pub(super) struct PlayerVolumeHint {
    pub(super) uuid: u128,
    pub(super) name: String,
    pub(super) at: Instant,
}
pub(super) struct HeadIconAnchor {
    pub(super) position: [f64; 3],
}
pub(super) struct State {
    pub(super) settings: Settings,
    client: Option<Client>,
    pub(super) status: Status,
    held: bool,
    pressed: HashSet<String>,
    ptt_suppressed: HashSet<String>,
    held_activations: HashSet<u128>,
    registered: bool,
    pending_payloads: VecDeque<Vec<u8>>,
    last_scene: Instant,
    last_occlusion: Instant,
    last_faces: Instant,
    last_diagnostic: Instant,
    occlusion: HashMap<u128, f32>,
    connection: Option<String>,
    pub(super) ui: crate::menu::Menu,
    pub(super) snapshot: Snapshot,
    pub(super) preview: bool,
    pub(super) error: String,
    pub(super) players: Vec<(u128, String)>,
    pub(super) faces: HashMap<u128, String>,
    pub(super) icons: HashMap<(bool, u128), [f32; 4]>,
    icon_hashes: HashMap<(bool, u128), [u8; 16]>,
    icon_pixels: Vec<u8>,
    icon_registries: (usize, usize),
    pub(super) name_icons: HashMap<u128, HeadIconAnchor>,
    pub(super) visualization: Option<Visualization>,
    pub(super) volume_hint: Option<PlayerVolumeHint>,
    visualize_join: bool,
    save_pending: bool,
    last_saved: Instant,
}
impl State {
    pub(super) fn ptt_held(&self) -> bool {
        self.held || !self.held_activations.is_empty()
    }
    fn new(game: &dyn Host) -> Self {
        let mut settings: Settings = game
            .setting(ID, "settings.json")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        settings.normalize();
        settings.language = horizon_client::lang::code();
        Self {
            settings,
            client: None,
            status: Status::Waiting,
            held: false,
            pressed: HashSet::new(),
            ptt_suppressed: HashSet::new(),
            held_activations: HashSet::new(),
            registered: false,
            pending_payloads: VecDeque::new(),
            last_scene: Instant::now(),
            last_occlusion: Instant::now(),
            last_faces: Instant::now() - Duration::from_secs(1),
            last_diagnostic: Instant::now(),
            occlusion: HashMap::new(),
            connection: None,
            ui: Default::default(),
            snapshot: Snapshot::default(),
            preview: false,
            error: String::new(),
            players: Vec::new(),
            faces: HashMap::new(),
            icons: HashMap::new(),
            icon_hashes: HashMap::new(),
            icon_pixels: Vec::new(),
            icon_registries: (0, 0),
            name_icons: HashMap::new(),
            visualization: None,
            volume_hint: None,
            visualize_join: false,
            save_pending: false,
            last_saved: Instant::now(),
        }
    }
}
fn states() -> &'static Mutex<HashMap<u64, State>> {
    static STATES: OnceLock<Mutex<HashMap<u64, State>>> = OnceLock::new();
    STATES.get_or_init(Mutex::default)
}
fn with_state(game: &mut dyn Host, f: impl FnOnce(&mut dyn Host, &mut State)) {
    if !game.window() {
        return;
    }
    let Some(id) = game.client_state().map(|v| v.window_id) else {
        return;
    };
    let Ok(mut states) = states().lock() else {
        return;
    };
    let state = states.entry(id).or_insert_with(|| State::new(game));
    f(game, state);
}
fn with_read_state(game: &dyn Host, f: impl FnOnce(&mut State)) {
    if !game.window() {
        return;
    }
    let Some(id) = game.client_state().map(|v| v.window_id) else {
        return;
    };
    if let Ok(mut states) = states().lock() {
        let state = states.entry(id).or_insert_with(|| State::new(game));
        f(state);
    }
}
fn on(game: &dyn Host) -> bool {
    game.on(ID)
}
struct Rsa(RsaPrivateKey);
impl KeyPair for Rsa {
    fn public_der(&self) -> Vec<u8> {
        self.0.public_der()
    }
    fn decrypt(&self, bytes: &[u8]) -> horizon_plasmo_engine::Result<Vec<u8>> {
        self.0.decrypt(bytes).map_err(|e| e.to_string())
    }
}
fn keypair() -> horizon_plasmo_engine::Result<Box<dyn KeyPair>> {
    Ok(Box::new(Rsa(
        RsaPrivateKey::generate(2048).map_err(|e| e.to_string())?
    )))
}
fn activation_id(name: &str, namespace: &str) -> u128 {
    let mut hash = horizon_protocol::crypto::md5(format!("{name}_{namespace}").as_bytes());
    hash[6] = (hash[6] & 15) | 0x30;
    hash[8] = (hash[8] & 63) | 0x80;
    u128::from_be_bytes(hash)
}
fn send(game: &mut dyn Host, channel: &str, data: Vec<u8>) -> bool {
    if let Err(error) = game.send_packet(Serverbound::CustomPayload(CustomPayload {
        channel: channel.into(),
        data,
    })) {
        log::debug!("Plasmo Voice payload deferred: {error}");
        return false;
    }
    true
}
fn flush_payloads(game: &mut dyn Host, queue: &mut VecDeque<Vec<u8>>) {
    while let Some(data) = queue.front() {
        if !send(game, protocol::CHANNEL, data.clone()) {
            break;
        }
        queue.pop_front();
    }
}
fn ensure(game: &mut dyn Host, state: &mut State) -> bool {
    if let Some(message) = unavailable_message() {
        state.client = None;
        state.status = Status::Error;
        state.error = message;
        return false;
    }
    if !on(game) || game.client_state().and_then(|v| v.server_address).is_none() {
        return false;
    }
    let Some(address) = game.client_state().and_then(|v| v.server_address) else {
        return false;
    };
    if state.connection.as_ref() != Some(&address) {
        state.client = None;
        state.registered = false;
        state.pending_payloads.clear();
        state.held = false;
    }
    if state.client.is_none() {
        let host = horizon_protocol::servers::parse_address(&address)
            .map(|v| v.0)
            .unwrap_or_else(|| "localhost".into());
        let devices = std::env::var_os("HORIZON_NO_AUDIO").is_none();
        match Client::start(
            host,
            keypair,
            activation_id,
            state.settings.clone(),
            devices,
        ) {
            Ok(client) => {
                state.client = Some(client);
                state.connection = Some(address);
                state.status = Status::Waiting;
            }
            Err(e) => {
                log::error!("Plasmo Voice worker: {e}");
                state.status = Status::Error;
                return false;
            }
        }
    }
    if !state.registered {
        state.registered = send(game, "minecraft:register", protocol::REGISTER.to_vec());
    }
    state.registered
}
pub(super) fn payload(game: &mut dyn Host, bytes: &[u8]) {
    with_state(game, |game, state| {
        if ensure(game, state) && !state.client.as_ref().unwrap().control(bytes) {
            log::warn!("Plasmo Voice control queue full; stopping this session");
            state.client = None;
            state.status = Status::Error;
        }
    });
}
pub(super) fn frame(game: &mut dyn Host) {
    with_state(game, |game, state| {
        actions(game, state);
        flush_settings(game, state);
        let language = horizon_client::lang::code();
        if state.settings.language != language {
            state.settings.language = language;
            if let Some(client) = &state.client {
                let _ = client.settings(state.settings.clone());
            }
        }
        refresh_icons(game, state);
        let own_screen = game.mod_screen() == Some(ID);
        let focused = game.client_state().is_some_and(|v| v.focused);
        if state.preview && (!own_screen || !focused) {
            state.preview = false;
            if let Some(client) = &state.client {
                client.preview(false);
            }
        }
        if game.client_state().and_then(|v| v.server_address).is_none() || !on(game) {
            state.client = None;
            state.registered = false;
            state.pending_payloads.clear();
            state.held = false;
            state.held_activations.clear();
            state.pressed.clear();
            state.ptt_suppressed.clear();
            state.connection = None;
            return;
        }
        if !game.client_state().is_some_and(|v| v.logged_in) || !ensure(game, state) {
            return;
        }
        if !focused {
            state.held = false;
            state.held_activations.clear();
            state.pressed.clear();
            state.ptt_suppressed.clear();
        }
        update_ptt(game, state);
        let client = state.client.as_ref().unwrap();
        if let Ok(mut keys) = client.keys.try_lock() {
            *keys = state.held_activations.clone();
        }
        client.metering.store(
            focused && own_screen && state.ui.devices_tab(),
            Ordering::Relaxed,
        );
        client
            .talking
            .store(state.held && !state.settings.muted, Ordering::Relaxed);
        client.active.store(
            focused
                && (game.playing() || state.ptt_held())
                && !state.preview
                && state.settings.enabled,
            Ordering::Relaxed,
        );
        let events: Vec<_> = client.events().collect();
        for event in events {
            match event {
                Event::Tcp(data) => {
                    // A server can temporarily leave play for configuration. Preserve ordering
                    // until Host accepts packets again, with a bounded control queue.
                    if state.pending_payloads.len() >= 128 {
                        state.error = "Voice control queue full".into();
                        log::warn!("Plasmo Voice: {}", state.error);
                        state.client = None;
                        state.status = Status::Error;
                        return;
                    }
                    state.pending_payloads.push_back(data);
                }
                Event::Status(status) => {
                    if status == Status::Connected
                        && state.status != Status::Connected
                        && state.settings.visualize_on_join
                    {
                        state.visualize_join = true;
                    }
                    state.status = status;
                }
                Event::ActionBar(json) => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) {
                        game.client_action_bar(horizon_client::scoreboard::comp_text(&value), true);
                    }
                }
                Event::Visualize {
                    radius,
                    color,
                    position,
                } => {
                    if let Some(center) =
                        position.or_else(|| game.player().map(|p| [p.x, p.y, p.z]))
                    {
                        state.visualization = Some(Visualization {
                            center,
                            radius: radius.clamp(1, 32767) as f32,
                            color,
                            at: Instant::now(),
                        });
                    }
                }
                Event::Error(error) => {
                    log::warn!("Plasmo Voice: {error}");
                    state.error = error;
                }
            }
        }
        flush_payloads(game, &mut state.pending_payloads);
        let client = state.client.as_ref().unwrap();
        if state.last_scene.elapsed() >= Duration::from_millis(20) {
            if let Some(snapshot) = client.snapshot() {
                state.snapshot = snapshot;
                static DIAGNOSTICS: OnceLock<bool> = OnceLock::new();
                if *DIAGNOSTICS
                    .get_or_init(|| std::env::var_os("HORIZON_PLASMO_DIAGNOSTICS").is_some())
                    && state.last_diagnostic.elapsed() >= Duration::from_millis(500)
                {
                    let v = &state.snapshot;
                    let data = serde_json::json!({"status":format!("{:?}",state.status),"sent":v.sent,"received":v.received,"level":v.level,"output_level":v.output_level,"transmitting":v.transmitting,"ptt_held":state.ptt_held(),"input_active":client.active.load(Ordering::Relaxed),"input_focused":focused,"microphone_open":v.microphone_open,"output_open":v.output_open,"sources":v.speakers.len(),"speakers":v.speakers.iter().map(|s|serde_json::json!({"source":s.source.to_string(),"player":s.player.to_string(),"name":s.name,"speaking":s.speaking,"icon_visible":s.icon_visible,"audible":s.audible})).collect::<Vec<_>>(),"hidden_icons":v.hidden_icons.as_ref(),"name_icons":state.name_icons.len(),"players":v.players.iter().map(|p|serde_json::json!({"name":p.name,"mic_muted":p.microphone_muted,"server_muted":p.server_muted,"voice_disabled":p.voice_disabled})).collect::<Vec<_>>(),"activations":v.activations.len(),"lines":v.lines.len(),"error":state.error});
                    game.set_setting(ID, "diagnostics.json", &data.to_string());
                    state.last_diagnostic = Instant::now();
                }
            }
            if state.visualize_join
                && state.snapshot.distance > 0
                && let Some(player) = game.player()
            {
                state.visualization = Some(Visualization {
                    center: [player.x, player.y, player.z],
                    radius: f32::from(state.snapshot.distance),
                    color: 0x55ff55,
                    at: Instant::now(),
                });
                state.visualize_join = false;
            }
            if state.last_faces.elapsed() >= Duration::from_millis(500) {
                let entities = game.client_entities();
                state.players = entities
                    .iter()
                    .filter_map(|e| Some((e.player_uuid?, e.player_name.clone()?)))
                    .collect();
                for speaker in &state.snapshot.speakers {
                    if !state.players.iter().any(|p| p.0 == speaker.player) {
                        state.players.push((speaker.player, speaker.name.clone()));
                    }
                }
                state.players.sort_by_cached_key(|a| a.1.to_lowercase());
                for p in state.snapshot.players.iter() {
                    if !state.players.iter().any(|v| v.0 == p.id) {
                        state.players.push((p.id, p.name.clone()));
                    }
                }
                state.faces.clear();
                for p in game.client_profiles() {
                    if own_screen || state.snapshot.speakers.iter().any(|s| s.player == p.uuid) {
                        state.faces.insert(p.uuid, p.skin);
                    }
                }
                state.last_faces = Instant::now();
            }
            let Some(player) = game.player() else {
                return;
            };
            let mut scene = Scene {
                listener: [player.x, player.y + 1.62, player.z],
                yaw: player.yaw,
                pitch: player.pitch,
                ..Scene::default()
            };
            if state.settings.camera_sound_listener
                && let Some(camera) = camera(game)
            {
                scene.listener = camera.position;
                scene.yaw = camera.yaw;
                scene.pitch = camera.pitch;
            }
            for entity in game.client_entities() {
                let [x, y, z] = entity.position;
                scene.entities.insert(entity.id as i32, [x, y + 1.0, z]);
                if let Some(uuid) = entity.player_uuid {
                    scene.players.insert(uuid, [x, y + 1.62, z]);
                    scene
                        .directions
                        .insert(uuid, crate::view_vector(entity.head_yaw, entity.head_pitch));
                }
            }
            if state.settings.sound_occlusion
                && state.last_occlusion.elapsed() >= Duration::from_millis(100)
            {
                state.occlusion.clear();
                for speaker in state
                    .snapshot
                    .speakers
                    .iter()
                    .filter(|s| s.speaking && s.audible)
                    .take(32)
                {
                    if let Some(position) = speaker.position {
                        let gain = horizon_plasmo_engine::occlusion::transmission(
                            scene.listener,
                            position,
                            |pos| {
                                let (full_collision, can_occlude) =
                                    game.client_block_obstruction(pos).unwrap_or((false, false));
                                if full_collision {
                                    0.45
                                } else if can_occlude {
                                    0.15
                                } else {
                                    0.
                                }
                            },
                        );
                        state.occlusion.insert(speaker.source, gain);
                    }
                }
                state.last_occlusion = Instant::now();
            }
            scene.occlusion = state.occlusion.clone();
            client.scene(scene);
            state.last_scene = Instant::now();
        }
    });
}
// Keys are observed on both ordinary gameplay and raw GUI input paths.
// Repeated observations are idempotent, and binding capture suppresses PTT
// until the captured keys have actually been released.
fn note_key(game: &dyn Host, state: &mut State, key: &str, pressed: bool) {
    if pressed {
        state.pressed.insert(key.to_owned());
        if game.mod_screen() == Some(ID) && state.ui.binding.is_some() {
            state.ptt_suppressed.insert(key.to_owned());
        }
    } else {
        state.pressed.remove(key);
        state.ptt_suppressed.remove(key);
        state.ui.key_release(key);
    }
}
fn update_ptt(game: &dyn Host, state: &mut State) {
    let allowed = state.ui.binding.is_none() && state.ptt_suppressed.is_empty();
    state.held = allowed
        && key_active(
            &game.mod_key(ID, "talk", "key.keyboard.left.alt"),
            &state.pressed,
        );
    state.held_activations.clear();
    if allowed {
        for (id, activation) in &state.settings.activations {
            if !activation.key.is_empty() && key_active(&activation.key, &state.pressed) {
                state.held_activations.insert(*id);
            }
        }
    }
}
pub(super) fn raw_key(game: &mut dyn Host, event: &winit::event::KeyEvent) {
    if !event.repeat
        && let winit::keyboard::PhysicalKey::Code(code) = event.physical_key
        && let Some(name) = horizon_client::keybinds::key_name(code)
    {
        key(
            game,
            name,
            event.state == winit::event::ElementState::Pressed,
        );
    }
}
pub(super) fn raw_mouse(game: &mut dyn Host, button: i32, pressed: bool) {
    let name = match button {
        0 => "key.mouse.left".to_owned(),
        1 => "key.mouse.right".to_owned(),
        2 => "key.mouse.middle".to_owned(),
        _ => format!("key.mouse.{}", button + 1),
    };
    key(game, &name, pressed);
}
pub(super) fn screen_key_release(game: &dyn Host, key: &str) {
    with_read_state(game, |state| {
        note_key(game, state, key, false);
        update_ptt(game, state);
    });
}
pub(super) fn key(game: &mut dyn Host, key: &str, pressed: bool) {
    let mute = game.mod_key(ID, "mute", "key.keyboard.m");
    let menu = game.mod_key(ID, "menu", "key.keyboard.v");
    let deafen = game.mod_key(ID, "deafen", "");
    with_state(game, |game, state| {
        if !on(game) {
            return;
        }
        let previous = state.pressed.clone();
        note_key(game, state, key, pressed);
        let current = state.pressed.clone();
        let active = |binding: &str| key_active(binding, &current);
        let trigger = |binding: &str| pressed && active(binding) && !key_active(binding, &previous);
        if trigger(&menu) && game.playing() && game.mod_screen_open(ID) {
            state.held = false;
            state.ui.reset();
            if std::env::var_os("HORIZON_NO_AUDIO").is_none()
                && let Some(client) = &state.client
            {
                client.refresh_devices();
            }
            return;
        }
        update_ptt(game, state);
        if trigger(&mute) && game.playing() {
            state.settings.muted = !state.settings.muted;
            persist(game, state);
        }
        if trigger(&deafen) && game.playing() {
            state.settings.enabled = !state.settings.enabled;
            persist(game, state);
        }
        if pressed && game.playing() {
            if trigger(&game.mod_key(ID, "occlusion", "")) {
                state.settings.sound_occlusion = !state.settings.sound_occlusion;
                persist(game, state);
            }
            let activations = state.snapshot.activations.clone();
            for activation in activations.iter() {
                for (prefix, direction) in [("increase", 1), ("decrease", -1)] {
                    if trigger(&game.mod_key(ID, &format!("{prefix}:{}", activation.id), "")) {
                        let current = horizon_plasmo_engine::activations::distance(
                            activation,
                            &state.settings,
                        );
                        let distance = if activation.dynamic_distance {
                            (i32::from(current) + direction).clamp(
                                1,
                                i32::from(activation.distances.last().copied().unwrap_or(32767)),
                            ) as u16
                        } else {
                            let mut distances = activation.distances.clone();
                            distances.sort_unstable();
                            if direction > 0 {
                                distances
                                    .into_iter()
                                    .find(|d| *d > current)
                                    .unwrap_or(current)
                            } else {
                                distances
                                    .into_iter()
                                    .rev()
                                    .find(|d| *d < current)
                                    .unwrap_or(current)
                            }
                        };
                        state
                            .settings
                            .activations
                            .entry(activation.id)
                            .or_default()
                            .distance = distance;
                        if activation.name == "proximity" {
                            state.settings.distance = distance;
                        }
                        show_distance(game, state, distance);
                        persist(game, state);
                    }
                }
            }
        }
    });
}
pub(super) fn disconnect(game: &dyn Host) {
    if let Some(id) = game.client_state().map(|v| v.window_id)
        && let Ok(mut states) = states().lock()
        && let Some(mut state) = states.remove(&id)
    {
        state.ui.reset();
        if state.save_pending
            && let Ok(json) = serde_json::to_string(&state.settings)
        {
            game.set_setting(ID, "settings.json", &json);
        }
    }
}
pub(super) fn tr(key: &str) -> String {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static RU: OnceLock<HashMap<String, String>> = OnceLock::new();
    let en = EN.get_or_init(|| {
        serde_json::from_slice(include_bytes!("../data/en_us.json")).expect("bundled translations")
    });
    let ru = RU.get_or_init(|| {
        serde_json::from_slice(include_bytes!("../data/ru_ru.json")).expect("bundled translations")
    });
    let strings = if horizon_client::lang::code().starts_with("ru") {
        ru
    } else {
        en
    };
    let fallback = strings
        .get(key)
        .or_else(|| en.get(key))
        .map(String::as_str)
        .unwrap_or(key);
    horizon_client::lang::get_or(&format!("plasmo_voice.{key}"), fallback)
}
pub(super) fn unavailable_message() -> Option<String> {
    horizon_plasmo_engine::audio_runtime::unavailable_reason()
        .map(|reason| tr(&format!("runtime_{reason}")))
}
pub(super) fn mods(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    mods.push(horizon_client::mod_api::NativeMod {
        id: ID.into(),
        title: "Plasmo Voice 0.5.4".into(),
        description: unavailable_message().map_or_else(
            || tr("description"),
            |message| format!("{}\n\n{message}", tr("description")),
        ),
        keys: vec![
            ["menu".into(), tr("open_menu"), "key.keyboard.v".into()],
            ["talk".into(), tr("talk"), "key.keyboard.left.alt".into()],
            ["mute".into(), tr("mute"), "key.keyboard.m".into()],
            ["deafen".into(), tr("disable_voice"), "".into()],
            ["action".into(), tr("action_key"), "key.mouse.right".into()],
            ["occlusion".into(), tr("toggle_occlusion"), "".into()],
        ],
    });
}
fn persist(game: &dyn Host, state: &mut State) {
    state.save_pending = true;
    if let Some(client) = &state.client {
        let _ = client.settings(state.settings.clone());
    }
    flush_settings(game, state);
}
fn show_distance(game: &dyn Host, state: &mut State, distance: u16) {
    if state.settings.visualize_distance
        && let Some(player) = game.player()
    {
        state.visualization = Some(Visualization {
            center: [player.x, player.y, player.z],
            radius: f32::from(distance),
            color: 0x55ff55,
            at: Instant::now(),
        });
    }
}
fn flush_settings(game: &dyn Host, state: &mut State) {
    if state.save_pending
        && (!state.ui.dragging() || state.last_saved.elapsed() >= Duration::from_millis(250))
    {
        if let Ok(json) = serde_json::to_string(&state.settings) {
            game.set_setting(ID, "settings.json", &json);
        }
        state.save_pending = false;
        state.last_saved = Instant::now();
    }
}
fn refresh_icons(game: &mut dyn Host, state: &mut State) {
    let registries = (
        std::sync::Arc::as_ptr(&state.snapshot.activations) as usize,
        std::sync::Arc::as_ptr(&state.snapshot.lines) as usize,
    );
    if registries == state.icon_registries {
        return;
    }
    state.icon_registries = registries;
    let entries: Vec<_> = state
        .snapshot
        .activations
        .iter()
        .map(|a| ((true, a.id), a.icon.clone()))
        .chain(
            state
                .snapshot
                .lines
                .iter()
                .map(|l| ((false, l.id), l.icon.clone())),
        )
        .collect();
    let active: HashSet<_> = entries.iter().map(|(key, _)| *key).collect();
    state.icons.retain(|key, _| active.contains(key));
    state.icon_hashes.retain(|key, _| active.contains(key));
    let mut changed = false;
    for (key, icon) in entries {
        let digest = horizon_protocol::crypto::md5(icon.as_bytes());
        if state.icon_hashes.get(&key) == Some(&digest) {
            continue;
        }
        state.icon_hashes.insert(key, digest);
        let existing = state.icons.remove(&key);
        if icon.len() > 131072 {
            continue;
        }
        let data = icon
            .strip_prefix("data:image/png;base64,")
            .or_else(|| icon.strip_prefix("base64:"))
            .unwrap_or(&icon);
        let Some(data) = horizon_client::skins::base64_decode(data) else {
            continue;
        };
        if data.len() < 24 || data[..8] != *b"\x89PNG\r\n\x1a\n" {
            continue;
        }
        let width = u32::from_be_bytes(data[16..20].try_into().unwrap());
        let height = u32::from_be_bytes(data[20..24].try_into().unwrap());
        if !(1..=256).contains(&width) || !(1..=256).contains(&height) {
            continue;
        }
        let Ok((width, height, rgba)) = horizon_client::hud::decode_image_bytes(&data) else {
            continue;
        };
        let rect = if let Some(rect) = existing {
            rect
        } else {
            let slot = (0..256usize).find(|slot| {
                !state
                    .icons
                    .values()
                    .any(|r| r[0] as usize / 16 + r[1] as usize / 16 * 16 == *slot)
            });
            let Some(slot) = slot else {
                continue;
            };
            [(slot % 16 * 16) as f32, (slot / 16 * 16) as f32, 16., 16.]
        };
        if state.icon_pixels.is_empty() {
            state.icon_pixels = vec![0; 256 * 256 * 4];
        }
        for y in 0..16usize {
            for x in 0..16usize {
                let source =
                    ((y * height as usize / 16) * width as usize + x * width as usize / 16) * 4;
                let dest = ((rect[1] as usize + y) * 256 + rect[0] as usize + x) * 4;
                state.icon_pixels[dest..dest + 4].copy_from_slice(&rgba[source..source + 4]);
            }
        }
        state.icons.insert(key, rect);
        changed = true;
    }
    if changed {
        game.mod_image(256, 256, &state.icon_pixels);
    }
}
pub(super) fn settings(_game: &mut dyn Host, buttons: &mut Vec<String>) {
    buttons.push(tr("open_menu"));
    if let Some(message) = unavailable_message() {
        buttons.push(message);
    }
}
pub(super) fn settings_click(game: &mut dyn Host, button: usize) {
    if button != 0 {
        return;
    }
    with_state(game, |game, state| {
        if game.mod_screen_open(ID) {
            state.ui.reset();
            state.held = false;
            if std::env::var_os("HORIZON_NO_AUDIO").is_none()
                && let Some(client) = &state.client
            {
                client.refresh_devices();
            }
        }
    });
}
pub(super) fn hud(game: &mut dyn Host, batch: &mut horizon_client::hud::HudBatch) {
    with_state(game, |game, state| {
        if horizon_plasmo_engine::audio_runtime::unavailable_reason().is_some()
            || !on(game)
            || game.client_state().and_then(|v| v.server_address).is_none()
        {
            return;
        }
        refresh_head_anchors(game, state);
        crate::menu::hud(game, state, batch);
    });
}

fn actions(game: &mut dyn Host, state: &mut State) {
    use crate::menu::{Action, Choice, Slider};
    use horizon_plasmo_engine::session::{
        IconVisibility, OverlayMode, OverlayStyle, SourceOverlap,
    };
    let actions = std::mem::take(&mut state.ui.pending);
    let mut changed = false;
    for action in actions {
        match action {
            Action::Close => {
                state.preview = false;
                if let Some(c) = &state.client {
                    c.preview(false);
                }
                state.ui.reset();
                game.mod_screen_close(ID);
            }
            Action::Preview => {
                state.preview = !state.preview;
                if let Some(c) = &state.client {
                    c.preview(state.preview);
                }
            }
            Action::Muted => {
                state.settings.muted = !state.settings.muted;
                changed = true;
            }
            Action::Enabled => {
                state.settings.enabled = !state.settings.enabled;
                changed = true;
            }
            Action::Input(name) => {
                state.settings.input_device = name;
                state.ui.reset();
                changed = true;
            }
            Action::Output(name) => {
                state.settings.output_device = name;
                state.ui.reset();
                changed = true;
            }
            Action::Bound(name, key) => {
                if let Some(id) = name
                    .strip_prefix("activation:")
                    .and_then(|s| s.parse::<u128>().ok())
                {
                    state.settings.activations.entry(id).or_default().key = key;
                    changed = true;
                } else {
                    game.set_mod_key(ID, &name, Some(&key));
                }
            }
            Action::ResetKey(name) => {
                if let Some(id) = name
                    .strip_prefix("activation:")
                    .and_then(|s| s.parse::<u128>().ok())
                {
                    state
                        .settings
                        .activations
                        .entry(id)
                        .or_default()
                        .key
                        .clear();
                    changed = true;
                } else {
                    game.set_mod_key(ID, &name, None);
                }
            }
            Action::Flag(flag, value) => {
                let value = value.unwrap_or_else(|| !flag.get(&state.settings));
                flag.set(&mut state.settings, value);
                changed = true;
            }
            Action::Anchor(hud, anchor) => {
                if hud {
                    state.settings.hud_position = anchor;
                } else {
                    state.settings.overlay_position = anchor;
                }
                changed = true;
            }
            Action::Choose(choice, index) => {
                match choice {
                    Choice::Input => state.settings.input_device.clear(),
                    Choice::Output => state.settings.output_device.clear(),
                    Choice::Mode(id) => {
                        let mode = [Mode::PushToTalk, Mode::Voice, Mode::Inherit][index.min(2)];
                        state.settings.activations.entry(id).or_default().mode = mode;
                        if state
                            .snapshot
                            .activations
                            .iter()
                            .any(|a| a.id == id && a.name == "proximity")
                        {
                            state.settings.mode = mode;
                        }
                    }
                    Choice::Icons => {
                        state.settings.icon_visibility = [
                            IconVisibility::GuiOnly,
                            IconVisibility::Always,
                            IconVisibility::Off,
                        ][index.min(2)]
                    }
                    Choice::Style => {
                        state.settings.overlay_style = [
                            OverlayStyle::NameSkin,
                            OverlayStyle::Name,
                            OverlayStyle::Skin,
                        ][index.min(2)]
                    }
                    Choice::Overlap => {
                        state.settings.overlap = [
                            SourceOverlap::MuteDirect,
                            SourceOverlap::MuteProximity,
                            SourceOverlap::Off,
                        ][index.min(2)]
                    }
                    Choice::LineOverlay(id) => {
                        state.settings.line_overlays.insert(
                            id,
                            [OverlayMode::Off, OverlayMode::Talking, OverlayMode::Always]
                                [index.min(2)],
                        );
                    }
                }
                changed = true;
            }
            Action::LineMute(id) => {
                if !state.settings.line_muted.remove(&id) {
                    state.settings.line_muted.insert(id);
                }
                changed = true;
            }
            Action::PlayerMute(id) => {
                if !state.settings.player_muted.remove(&id) {
                    state.settings.player_muted.insert(id);
                }
                changed = true;
            }
            Action::Value(slider, value) => {
                match slider {
                    Slider::Gain => state.settings.microphone_gain = value,
                    Slider::Volume => state.settings.volume = value,
                    Slider::Threshold => state.settings.threshold_db = value,
                    Slider::Angle => state.settings.directional_angle = value,
                    Slider::Line(id) => {
                        state.settings.line_volumes.insert(id, value);
                    }
                    Slider::Distance(id, _, _) => {
                        if let Some(a) = state.snapshot.activations.iter().find(|a| a.id == id) {
                            let requested = value.round().clamp(1., 32767.) as u16;
                            let distance = if a.dynamic_distance {
                                requested.clamp(1, a.distances.last().copied().unwrap_or(32767))
                            } else {
                                a.distances
                                    .iter()
                                    .copied()
                                    .min_by_key(|d| d.abs_diff(requested))
                                    .unwrap_or(a.default_distance)
                            };
                            state.settings.activations.entry(id).or_default().distance = distance;
                            if a.name == "proximity" {
                                state.settings.distance = distance;
                            }
                            show_distance(game, state, distance);
                        }
                    }
                    Slider::Player(uuid) => {
                        state.settings.player_volumes.insert(uuid, value);
                    }
                }
                changed = true;
            }
            _ => {}
        }
    }
    if changed {
        state.settings.normalize();
        persist(game, state);
    }
}
pub(super) fn render(
    game: &dyn Host,
    batch: &mut horizon_client::hud::HudBatch,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
) {
    if game.mod_screen() == Some(ID) {
        with_read_state(game, |state| {
            crate::menu::render(game, state, batch, screen)
        });
    }
}
pub(super) fn mouse(
    game: &dyn Host,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    pressed: bool,
    canceled: &mut bool,
) {
    if game.mod_screen() == Some(ID) {
        *canceled = true;
        if button == 0 {
            with_read_state(game, |state| state.ui.mouse(screen.mouse, pressed));
        } else if pressed {
            with_read_state(game, |state| {
                if state.ui.binding.is_some() {
                    let name = if button == 1 {
                        "key.mouse.right".into()
                    } else if button == 2 {
                        "key.mouse.middle".into()
                    } else {
                        format!("key.mouse.{}", button + 1)
                    };
                    state.ui.bind_key(&name);
                }
            });
        }
    }
}
pub(super) fn drag(
    game: &dyn Host,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    canceled: &mut bool,
) {
    if game.mod_screen() == Some(ID) {
        *canceled = true;
        with_read_state(game, |state| state.ui.dragged(screen.mouse));
    }
}
// Use the closest entity under the crosshair, limited by the first solid hit.
// Picking a player behind another entity or through a wall must not change volume.
fn volume_target(game: &dyn Host) -> Option<(u128, String)> {
    let context = game.client_state()?;
    let eye = context.eye_position?;
    let direction = crate::view_vector(context.yaw, context.pitch);
    let reach = game.raycast(32.0).map_or(32.0, |hit| {
        ([hit.x, hit.y, hit.z]
            .into_iter()
            .zip(eye)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>())
        .sqrt()
    });
    let mob = game
        .client_entities()
        .into_iter()
        .filter(|m| !m.invisible)
        .filter_map(|m| crate::ray_box(eye, direction, reach, m.bounds).map(|d| (m, d)))
        .min_by(|a, b| a.1.total_cmp(&b.1))?
        .0;
    Some((mob.player_uuid?, mob.player_name?))
}
pub(super) fn wheel(game: &mut dyn Host, steps: f32, claimed: &mut bool) {
    if !on(game) || !game.playing() || !steps.is_finite() || steps == 0. {
        return;
    }
    let binding = game.mod_key(ID, "action", "key.mouse.right");
    with_state(game, |game, state| {
        if !key_active(&binding, &state.pressed)
            || !(state.pressed.contains("key.keyboard.left.shift")
                || state.pressed.contains("key.keyboard.right.shift"))
        {
            return;
        }
        let Some((uuid, name)) = volume_target(game) else {
            return;
        };
        *claimed = true;
        let previous = state
            .settings
            .player_volumes
            .get(&uuid)
            .copied()
            .unwrap_or(1.);
        let volume = ((previous + steps * 0.05).clamp(0., 2.) * 1000.).round() / 1000.;
        if volume == previous {
            return;
        }
        state.settings.player_volumes.insert(uuid, volume);
        state.volume_hint = Some(PlayerVolumeHint {
            uuid,
            name,
            at: Instant::now(),
        });
        persist(game, state);
    });
}
pub(super) fn scroll(game: &dyn Host, steps: f64, canceled: &mut bool) {
    if game.mod_screen() == Some(ID) {
        *canceled = true;
        with_read_state(game, |state| state.ui.wheel(steps));
    }
}
pub(super) fn screen_key(game: &dyn Host, key: &str, repeat: bool, canceled: &mut bool) {
    with_read_state(game, |state| {
        if !repeat {
            note_key(game, state, key, true);
            update_ptt(game, state);
        }
        if game.mod_screen() == Some(ID) {
            *canceled = true;
            if !repeat {
                let menu = game.mod_key(ID, "menu", "key.keyboard.v");
                state.ui.key(key, &menu);
            }
        }
    });
}
pub(super) fn character(game: &dyn Host, c: char, canceled: &mut bool) {
    if game.mod_screen() == Some(ID) {
        *canceled = true;
        with_read_state(game, |state| state.ui.character(c));
    }
}
pub(super) const HEAD_ICON_SIZE: f32 = 0.32;
const HEAD_ICON_GAP: f64 = 0.18;
const NAME_ICON_GAP: f64 = 0.08;

pub(super) fn foreground_hud(game: &dyn Host, batch: &mut horizon_client::hud::HudBatch) {
    if horizon_plasmo_engine::audio_runtime::unavailable_reason().is_some()
        || !on(game)
        || game.client_state().and_then(|v| v.server_address).is_none()
    {
        return;
    }
    with_read_state(game, |state| {
        let hidden = game.client_state().is_some_and(|v| v.gui_hidden);
        if hidden && game.playing() {
            if state.settings.icon_visibility
                == horizon_plasmo_engine::session::IconVisibility::Always
            {
                refresh_head_anchors(game, state);
                crate::menu::world_icons(game, state, batch);
            }
        } else {
            crate::menu::activation_hud(game, state, batch);
        }
    });
}

// HUD extraction precedes entity geometry/name tags in Game::render. Only keep
// the text width from that pass: interpolate head positions with THIS frame's
// world fraction, including F1, rather than drawing last frame's anchors.
fn refresh_head_anchors(game: &dyn Host, state: &mut State) {
    state.name_icons.clear();
    let Some(context) = game.client_state() else {
        return;
    };
    let Some(camera) = game.camera() else {
        return;
    };
    let entities = game.client_entities();
    let ids: Vec<_> = entities
        .iter()
        .filter(|m| m.alive && !m.invisible && m.player_uuid.is_some())
        .filter(|m| {
            m.position
                .iter()
                .zip(camera.position)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                < 64.0 * 64.0
        })
        .map(|m| m.id)
        .collect();
    for head in game.client_heads(&ids, context.world_partial_tick) {
        let Some(uuid) = entities
            .iter()
            .find(|m| m.id == head.id)
            .and_then(|m| m.player_uuid)
        else {
            continue;
        };
        state.name_icons.insert(
            uuid,
            HeadIconAnchor {
                position: head_icon_position(&head),
            },
        );
    }
}

fn head_icon_position(head: &NativeClientHead) -> [f64; 3] {
    let at = glam::DVec3::from_array(head.origin);
    let mut center = at + glam::DVec3::Y * f64::from(head.height);
    let mut top = center.y;
    if let Some(matrix) = head.transform {
        let pose = glam::Mat4::from_cols_array(&matrix);
        center = at
            + pose
                .transform_point3(glam::Vec3::new(0.0, -0.25, 0.0))
                .as_dvec3();
        let padding = head.outer_layer_padding;
        top = f64::NEG_INFINITY;
        for x in [-0.25 - padding, 0.25 + padding] {
            for y in [-0.5 - padding, padding] {
                for z in [-0.25 - padding, 0.25 + padding] {
                    top = top
                        .max(at.y + f64::from(pose.transform_point3(glam::Vec3::new(x, y, z)).y));
                }
            }
        }
    }
    let half_icon = f64::from(HEAD_ICON_SIZE) * 0.5;
    center.y = top + HEAD_ICON_GAP + half_icon;
    if head.name_half_width > 0.0 {
        center.y = center
            .y
            .max(head.name_anchor[1] + 0.025 + NAME_ICON_GAP + half_icon);
    }
    center.to_array()
}

// The public camera snapshot is this frame during GUI extraction (PR #9556).
pub(super) fn camera(game: &dyn Host) -> Option<horizon_client::mod_api::NativeCamera> {
    game.camera()
}

fn key_active(binding: &str, pressed: &HashSet<String>) -> bool {
    !binding.is_empty() && binding.split('+').all(|key| pressed.contains(key.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_client::mod_api::{NativeClientEntity, NativeClientState, NativeHit};
    use std::cell::RefCell;

    #[derive(Default)]
    struct TestHost {
        id: u64,
        playing: bool,
        entities: Vec<NativeClientEntity>,
        wall: Option<f64>,
        reject_packets: bool,
        sent: Vec<Vec<u8>>,
        saved: RefCell<Vec<String>>,
        fills: RefCell<Vec<([f32; 4], u32)>>,
    }
    impl Host for TestHost {
        fn on(&self, _: &str) -> bool {
            true
        }
        fn playing(&self) -> bool {
            self.playing
        }
        fn window(&self) -> bool {
            true
        }
        fn client_state(&self) -> Option<NativeClientState> {
            let mut state = NativeClientState::default();
            state.window_id = self.id;
            state.eye_position = Some([0., 1.62, 0.]);
            state.gui_size = [640., 360.];
            Some(state)
        }
        fn fill(&self, _: &mut horizon_client::hud::HudBatch, rect: [f32; 4], color: u32) {
            self.fills.borrow_mut().push((rect, color));
        }
        fn client_entities(&self) -> Vec<NativeClientEntity> {
            self.entities.clone()
        }
        fn raycast(&self, _: f32) -> Option<NativeHit> {
            self.wall.map(|z| NativeHit {
                block_x: 0,
                block_y: 1,
                block_z: z as i32,
                x: 0.,
                y: 1.62,
                z,
                block: "minecraft:stone".into(),
            })
        }
        fn send_packet(&mut self, packet: Serverbound) -> Result<(), String> {
            if self.reject_packets {
                return Err("temporarily reconfiguring".into());
            }
            let Serverbound::CustomPayload(payload) = packet else {
                panic!("unexpected packet");
            };
            assert_eq!(payload.channel, protocol::CHANNEL);
            self.sent.push(payload.data);
            Ok(())
        }
        fn setting(&self, _: &str, _: &str) -> Option<String> {
            None
        }
        fn set_setting(&self, _: &str, _: &str, value: &str) {
            self.saved.borrow_mut().push(value.into());
        }
        fn mod_screen(&self) -> Option<&str> {
            None
        }
        fn mod_key(&self, _: &str, _: &str, default: &str) -> String {
            default.into()
        }
    }

    #[test]
    fn disabled_build_explains_reason_without_registering_voice() {
        let Some(message) = unavailable_message() else {
            return;
        };
        let mut host = TestHost::default();
        let mut state = State::new(&host);
        assert!(!ensure(&mut host, &mut state));
        assert!(state.client.is_none());
        assert!(host.sent.is_empty());
        assert_eq!(state.error, message);
        let mut entries = Vec::new();
        mods(&mut entries);
        assert!(entries[0].description.contains(&message));
        let mut buttons = Vec::new();
        settings(&mut host, &mut buttons);
        assert_eq!(buttons[1], message);
    }

    #[test]
    fn activation_hud_hides_on_ordinary_servers_and_stops_animating_after_failure() {
        let host = TestHost::default();
        let mut state = State::new(&host);
        let mut batch = horizon_client::hud::HudBatch::new([1280, 720]);
        state.held = true;
        state.settings.muted = true;
        crate::menu::activation_hud(&host, &state, &mut batch);
        assert!(host.fills.borrow().is_empty());

        state.settings.muted = false;
        state.held = false;
        let draw = |state: &State| {
            host.fills.borrow_mut().clear();
            let mut batch = horizon_client::hud::HudBatch::new([1280, 720]);
            crate::menu::activation_hud(&host, state, &mut batch);
            host.fills.borrow().clone()
        };
        state.status = Status::Connecting;
        let connecting = draw(&state);
        assert!(!connecting.is_empty());
        state.status = Status::TimedOut;
        let failed = draw(&state);
        assert!(!failed.is_empty());
        assert_ne!(failed, connecting);
        state.status = Status::Error;
        assert_eq!(draw(&state), failed);
        state.status = Status::ListenOnly;
        let microphone_unavailable = draw(&state);
        assert!(!microphone_unavailable.is_empty());
        assert_ne!(microphone_unavailable, connecting);
        assert_ne!(microphone_unavailable, failed);
        state.status = Status::Connected;
        assert!(draw(&state).is_empty());
        state.held = true;
        assert!(!draw(&state).is_empty());
        state.status = Status::Waiting;
        assert!(draw(&state).is_empty());
    }

    #[test]
    fn control_messages_survive_reconfiguration_in_order_without_duplicates() {
        let mut host = TestHost {
            reject_packets: true,
            ..Default::default()
        };
        let mut pending = VecDeque::from([vec![1], vec![2], vec![3]]);
        flush_payloads(&mut host, &mut pending);
        assert!(host.sent.is_empty());
        assert_eq!(pending.len(), 3);
        host.reject_packets = false;
        flush_payloads(&mut host, &mut pending);
        flush_payloads(&mut host, &mut pending);
        assert_eq!(host.sent, [vec![1], vec![2], vec![3]]);
        assert!(pending.is_empty());
    }

    #[test]
    fn player_volume_selection_respects_nearer_entities_and_blocks() {
        let entity = |z: f64, player| {
            let mut entity = NativeClientEntity::default();
            entity.bounds = [[-0.3, 0., z - 0.3], [0.3, 1.8, z + 0.3]];
            entity.player_uuid = player;
            entity.player_name = player.map(|id| format!("Player{id}"));
            entity
        };
        let player = entity(6., Some(7));
        let mut host = TestHost {
            entities: vec![player.clone()],
            ..Default::default()
        };
        assert_eq!(volume_target(&host), Some((7, "Player7".into())));
        host.wall = Some(4.);
        assert_eq!(volume_target(&host), None);
        host.wall = None;
        host.entities.push(entity(3., None));
        assert_eq!(volume_target(&host), None);
        host.entities = vec![player, entity(3., Some(8))];
        assert_eq!(volume_target(&host), Some((8, "Player8".into())));
        host.entities[1].invisible = true;
        assert_eq!(volume_target(&host), Some((7, "Player7".into())));
    }

    #[test]
    fn player_volume_hint_requires_shift_action_scroll_and_an_actual_change() {
        let mut player = NativeClientEntity::default();
        player.bounds = [[-0.3, 0., 5.7], [0.3, 1.8, 6.3]];
        player.player_uuid = Some(7);
        player.player_name = Some("Player7".into());
        let mut host = TestHost {
            id: 90002,
            playing: true,
            entities: vec![player],
            ..Default::default()
        };
        raw_mouse(&mut host, 1, true);
        frame(&mut host);
        assert!(states().lock().unwrap()[&host.id].volume_hint.is_none());

        // Plain scrolling, RMB scrolling and Shift scrolling retain normal game behavior.
        for keys in [
            vec![],
            vec!["key.mouse.right"],
            vec!["key.keyboard.left.shift"],
        ] {
            states().lock().unwrap().get_mut(&host.id).unwrap().pressed =
                keys.into_iter().map(str::to_owned).collect();
            let mut claimed = false;
            wheel(&mut host, 1., &mut claimed);
            assert!(!claimed);
            let all = states().lock().unwrap();
            assert!(all[&host.id].settings.player_volumes.is_empty());
            assert!(all[&host.id].volume_hint.is_none());
        }

        for shift in ["key.keyboard.left.shift", "key.keyboard.right.shift"] {
            {
                let mut all = states().lock().unwrap();
                let state = all.get_mut(&host.id).unwrap();
                state.pressed = ["key.mouse.right", shift]
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                state.volume_hint = None;
            }
            let mut claimed = false;
            wheel(&mut host, 1., &mut claimed);
            assert!(claimed);
            {
                let all = states().lock().unwrap();
                assert_eq!(all[&host.id].settings.player_volumes[&7], 1.05);
                let hint = all[&host.id].volume_hint.as_ref().unwrap();
                assert_eq!((hint.uuid, hint.name.as_str()), (7, "Player7"));
            }
            wheel(&mut host, -1., &mut claimed);
            assert_eq!(
                states().lock().unwrap()[&host.id].settings.player_volumes[&7],
                1.
            );
        }

        {
            let mut all = states().lock().unwrap();
            let state = all.get_mut(&host.id).unwrap();
            state.settings.player_volumes.insert(7, 2.);
            state.volume_hint = None;
        }
        let mut claimed = false;
        wheel(&mut host, 1., &mut claimed);
        assert!(claimed); // Keep the hotbar still at the volume limit.
        assert!(states().lock().unwrap()[&host.id].volume_hint.is_none());
        host.wall = Some(4.);
        claimed = false;
        wheel(&mut host, -1., &mut claimed);
        assert!(!claimed);
        assert_eq!(
            states().lock().unwrap()[&host.id].settings.player_volumes[&7],
            2.
        );
        assert!(states().lock().unwrap()[&host.id].volume_hint.is_none());
        disconnect(&host);
    }

    #[test]
    fn posed_helmet_has_fixed_world_clearance_and_tracks_crouching() {
        let mut head = NativeClientHead::default();
        head.origin = [10., 64., 20.];
        head.outer_layer_padding = 1.5 / 16.;
        head.transform =
            Some(glam::Mat4::from_translation(glam::Vec3::new(0., 2., 0.)).to_cols_array());
        let standing = head_icon_position(&head);
        assert!((standing[0] - 10.).abs() < 1e-6 && (standing[2] - 20.).abs() < 1e-6);
        assert!((standing[1] - 66.43375).abs() < 1e-6);
        head.transform =
            Some(glam::Mat4::from_translation(glam::Vec3::new(0., 1.5, 0.)).to_cols_array());
        let crouching = head_icon_position(&head);
        assert!((standing[1] - crouching[1] - 0.5).abs() < 1e-6);
        head.name_half_width = 0.5;
        head.name_anchor = [10., 67., 20.];
        assert!((head_icon_position(&head)[1] - 67.265).abs() < 1e-6);
    }

    #[test]
    fn gui_ptt_receives_release_and_disconnect_saves_pending_changes_once() {
        // playing() is false: these GUI callbacks must not depend on gameplay focus.
        let host = TestHost {
            id: 90001,
            ..Default::default()
        };
        let mut canceled = false;
        screen_key(&host, "key.keyboard.left.alt", false, &mut canceled);
        assert!(states().lock().unwrap()[&host.id].ptt_held());
        screen_key_release(&host, "key.keyboard.left.alt");
        assert!(!states().lock().unwrap()[&host.id].ptt_held());
        {
            let mut all = states().lock().unwrap();
            let state = all.get_mut(&host.id).unwrap();
            state.settings.player_volumes.insert(7, 1.75);
            state.save_pending = true;
            state.last_saved = Instant::now();
        }
        disconnect(&host);
        disconnect(&host);
        let saved = host.saved.borrow();
        assert_eq!(saved.len(), 1);
        let settings: Settings = serde_json::from_str(&saved[0]).unwrap();
        assert_eq!(settings.player_volumes[&7], 1.75);
        assert!(!states().lock().unwrap().contains_key(&host.id));
    }
}
