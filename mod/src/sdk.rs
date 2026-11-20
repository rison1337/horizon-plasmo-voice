//! The only module that reaches Horizon: public client SDK services and hooks.
use crate::ui::{self, Host as VoiceHost};
use mod_api::{self, ClientHost, DrawContext, Text};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Mutex, OnceLock},
    time::Instant,
};

struct Track {
    tick: i64,
    previous: [f64; 3],
    current: [f64; 3],
}
fn positions() -> &'static Mutex<HashMap<mod_api::EntityRef, Track>> {
    static DATA: OnceLock<Mutex<HashMap<mod_api::EntityRef, Track>>> = OnceLock::new();
    DATA.get_or_init(Mutex::default)
}
fn previous(entity: mod_api::EntityRef, position: [f64; 3], tick: i64) -> [f64; 3] {
    let mut data = positions().lock().unwrap();
    if data.len() > 4096 {
        data.clear();
    }
    let v = data.entry(entity).or_insert(Track {
        tick,
        previous: position,
        current: position,
    });
    if v.tick != tick {
        v.previous = v.current;
        v.tick = tick;
    }
    v.current = position;
    v.previous
}
fn overlay() -> &'static Mutex<Option<(String, Instant)>> {
    static VALUE: OnceLock<Mutex<Option<(String, Instant)>>> = OnceLock::new();
    VALUE.get_or_init(Mutex::default)
}

enum Access<'a> {
    Read(&'a dyn ClientHost),
    Write(&'a mut dyn ClientHost),
}
pub struct Bridge<'a> {
    access: Access<'a>,
    size: [f32; 2],
}
fn view_size() -> &'static Mutex<[f32; 2]> {
    static SIZE: OnceLock<Mutex<[f32; 2]>> = OnceLock::new();
    SIZE.get_or_init(|| Mutex::new([854., 480.]))
}
impl Bridge<'_> {
    fn host(&self) -> &dyn ClientHost {
        match &self.access {
            Access::Read(h) => *h,
            Access::Write(h) => &**h,
        }
    }
    fn writable(&mut self) -> Result<&mut dyn ClientHost, String> {
        match &mut self.access {
            Access::Write(h) => Ok(&mut **h),
            Access::Read(_) => Err("read-only client event".into()),
        }
    }
    fn initialize(&self) {
        ui::lang::set(
            self.host()
                .client_session()
                .client_language()
                .unwrap_or_else(|| "en_us".into()),
        );
        let _ = crate::voice_engine::audio_runtime::configure(
            self.host().instance_dir().join(".plasmo_voice/audio"),
        );
        crate::voice_engine::audio_runtime::request_prepare();
    }
    pub(crate) fn read(host: &dyn ClientHost) -> Bridge<'_> {
        let v = Bridge {
            access: Access::Read(host),
            size: *view_size().lock().unwrap(),
        };
        v.initialize();
        v
    }
    pub(crate) fn write(host: &mut dyn ClientHost) -> Bridge<'_> {
        let v = Bridge {
            access: Access::Write(host),
            size: *view_size().lock().unwrap(),
        };
        v.initialize();
        v
    }
}
impl VoiceHost for Bridge<'_> {
    fn world_voice_icons(&self) -> bool {
        true
    }
    fn playing(&self) -> bool {
        self.host().playing()
    }
    fn on(&self, id: &str) -> bool {
        self.host().on(id)
    }
    fn client_state(&self) -> Option<ui::NativeClientState> {
        let h = self.host();
        let camera = h.camera();
        let mut hash = std::hash::DefaultHasher::new();
        h.instance_dir().hash(&mut hash);
        Some(ui::NativeClientState {
            window_id: hash.finish(),
            focused: h.window_focused(),
            gui_hidden: !matches!(h.hud_mode(), mod_api::HudMode::AllVisible),
            gui_size: self.size,
            server_address: h.client_session().client_server().and_then(|v| v.address),
            logged_in: h.local_player().is_some(),
            world_partial_tick: camera.as_ref().map_or(1., |v| f64::from(v.partial_tick)),
            eye_position: camera.as_ref().map(|v| v.position),
            yaw: camera.as_ref().map_or(0., |v| v.yaw),
            pitch: camera.as_ref().map_or(0., |v| v.pitch),
        })
    }
    fn camera(&self) -> Option<ui::NativeCamera> {
        self.host().camera()
    }
    fn player(&self) -> Option<ui::Player> {
        let h = self.host();
        let entity = h.local_player()?.entity();
        let [x, y, z] = entity.position(h)?;
        let [yaw, pitch] = entity.rotation(h).unwrap_or([0., 0.]);
        Some(ui::Player {
            x,
            y,
            z,
            yaw,
            pitch,
        })
    }
    fn client_entities(&self) -> Vec<ui::NativeClientEntity> {
        let h = self.host();
        let Some(camera) = h.camera() else {
            return vec![];
        };
        let profiles = h
            .client_session()
            .client_tab_list()
            .map(|v| v.entries)
            .unwrap_or_default();
        let players: HashMap<_, _> = profiles
            .iter()
            .filter_map(|p| Some((h.player(p.uuid)?.entity().id(), (p.uuid, p.name.clone()))))
            .collect();
        let min = camera.position.map(|v| v - 96.);
        let max = camera.position.map(|v| v + 96.);
        let level = h.level(&camera.dimension);
        let tick = level.and_then(|l| l.game_time(h)).unwrap_or(0);
        let entities = level
            .and_then(|l| l.entities_in(h, min, max))
            .unwrap_or_default();
        entities
            .into_iter()
            .filter_map(|entity| {
                let position = entity.position(h)?;
                let player = players.get(&entity.id());
                let [yaw, pitch] = entity.rotation(h).unwrap_or([0., 0.]);
                let bounds = entity.bounding_box(h).unwrap_or([position, position]);
                Some(ui::NativeClientEntity {
                    id: entity.id(),
                    player_uuid: player.map(|p| p.0),
                    player_name: player.map(|p| p.1.clone()),
                    previous_position: previous(entity, position, tick),
                    position,
                    bounds,
                    height: (bounds[1][1] - bounds[0][1]) as f32,
                    head_yaw: entity.head_yaw(h).unwrap_or(yaw),
                    head_pitch: pitch,
                    alive: h.living(entity).and_then(|l| l.alive(h)).unwrap_or(true),
                    invisible: entity.is_invisible(h).unwrap_or(false),
                })
            })
            .collect()
    }
    fn client_heads(&self, ids: &[u32], partial: f64) -> Vec<ui::NativeClientHead> {
        self.client_entities()
            .into_iter()
            .filter(|e| ids.contains(&e.id))
            .map(|e| {
                let origin = std::array::from_fn(|i| {
                    e.previous_position[i]
                        + (e.position[i] - e.previous_position[i]) * partial.clamp(0., 1.)
                });
                ui::NativeClientHead {
                    id: e.id,
                    origin,
                    height: e.height + 0.08,
                    outer_layer_padding: 0.0625,
                    name_anchor: [origin[0], origin[1] + f64::from(e.height) + 0.5, origin[2]],
                    name_half_width: e
                        .player_name
                        .as_ref()
                        .map_or(0., |n| self.text_width(n) * 0.0125),
                    ..Default::default()
                }
            })
            .collect()
    }
    fn raycast(&self, distance: f32) -> Option<ui::NativeHit> {
        let hit = self.host().raycast(distance)?;
        Some(ui::NativeHit {
            x: hit.x,
            y: hit.y,
            z: hit.z,
        })
    }
    fn client_block_obstruction(&self, pos: [i32; 3]) -> Option<(bool, bool)> {
        let h = self.host();
        let level = h.level(h.dimension())?;
        let block = level.block(h, pos)?;
        let id = block.id(h);
        let boxes = h.block_collision(pos);
        let full = boxes
            .iter()
            .any(|b| (b[3] - b[0]) * (b[4] - b[1]) * (b[5] - b[2]) >= 0.999);
        Some((full, !transparent(&id)))
    }
    fn client_visual_clear(&self, from: [f64; 3], to: [f64; 3]) -> bool {
        let h = self.host();
        let Some(level) = h.level(h.dimension()) else {
            return false;
        };
        let direction: [f64; 3] = std::array::from_fn(|i| to[i] - from[i]);
        let mut start = from;
        for _ in 0..64 {
            let Some(hit) = level.clip(
                h,
                start,
                to,
                mod_api::ClipBlock::Outline,
                mod_api::ClipFluid::None,
            ) else {
                return false;
            };
            if hit.miss {
                return true;
            }
            if hit.world_border {
                return false;
            }
            let Some(block) = level.block(h, hit.block) else {
                return false;
            };
            if !transparent(&block.id(h)) {
                return false;
            }
            let exit = (0..3)
                .filter(|i| direction[*i].abs() > 1e-9)
                .map(|i| {
                    let edge = f64::from(hit.block[i]) + if direction[i] > 0. { 1. } else { 0. };
                    (edge - from[i]) / direction[i]
                })
                .filter(|t| *t >= 0.)
                .fold(f64::INFINITY, f64::min)
                + 1e-6;
            if exit >= 1. {
                return true;
            }
            start = std::array::from_fn(|i| from[i] + direction[i] * exit);
        }
        false
    }
    fn client_profiles(&self) -> Vec<ui::NativeClientProfile> {
        self.host()
            .client_session()
            .client_tab_list()
            .map(|v| {
                v.entries
                    .into_iter()
                    .map(|p| ui::NativeClientProfile {
                        uuid: p.uuid,
                        skin: format!("entity:{}", p.uuid),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn setting(&self, id: &str, key: &str) -> Option<String> {
        self.host().setting(id, key)
    }
    fn set_setting(&self, id: &str, key: &str, value: &str) {
        self.host().set_setting(id, key, value);
    }
    fn mod_key(&self, id: &str, key: &str, default: &str) -> String {
        let saved: HashMap<String, String> = self
            .setting(id, "keys.json")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        if let Some(v) = saved.get(key) {
            return v.clone();
        }
        self.host()
            .mod_key(id, key)
            .map(|(key, m)| {
                let mut keys = Vec::new();
                if m.bits & 1 != 0 {
                    keys.push("key.keyboard.left.shift".into())
                }
                if m.bits & 2 != 0 {
                    keys.push("key.keyboard.left.control".into())
                }
                if m.bits & 4 != 0 {
                    keys.push("key.keyboard.left.alt".into())
                }
                keys.push(key);
                keys.join("+")
            })
            .unwrap_or_else(|| default.into())
    }
    fn set_mod_key(&mut self, id: &str, key: &str, value: Option<&str>) {
        let mut saved: HashMap<String, String> = self
            .setting(id, "keys.json")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        if let Some(value) = value {
            saved.insert(key.into(), value.into());
        } else {
            saved.remove(key);
        }
        self.set_setting(id, "keys.json", &serde_json::to_string(&saved).unwrap());
    }
    fn mod_screen(&self) -> Option<&str> {
        self.host().mod_screen()
    }
    fn mod_screen_open(&mut self, id: &str) -> bool {
        self.writable().is_ok_and(|h| h.mod_screen_open(id))
    }
    fn mod_screen_close(&mut self, id: &str) {
        if let Ok(h) = self.writable() {
            h.mod_screen_close(id)
        }
    }
    fn mod_image(&mut self, w: u32, h: u32, rgba: &[u8]) -> bool {
        self.writable().is_ok_and(|host| host.mod_image(w, h, rgba))
    }
    fn text_width(&self, s: &str) -> f32 {
        self.host().text_width(&Text::literal(s)).unwrap_or(0) as f32
    }
    fn client_action_bar(&mut self, json: String, _animate: bool) {
        *overlay().lock().unwrap() = Some((json, Instant::now()));
    }
    fn send_packet(
        &mut self,
        packet: crate::network::packets::play::Serverbound,
    ) -> Result<(), String> {
        let crate::network::packets::play::Serverbound::CustomPayload(p) = packet;
        self.writable()?
            .send_packet(crate::network::plugin_packet(&p.channel, &p.data)?)
            .map_err(|e| format!("{e:?}"))
    }
}

pub(crate) fn paint(
    host: &dyn ClientHost,
    draw: &mut dyn DrawContext,
    render: impl FnOnce(&mut Bridge<'_>, &mut ui::HudBatch),
) {
    let size = draw.size();
    *view_size().lock().unwrap() = size;
    let mut bridge = Bridge::read(host);
    bridge.size = size;
    ui::widgets::take();
    let mut batch = ui::HudBatch::new(size.map(|v| v.max(0.) as u32));
    render(&mut bridge, &mut batch);
    // Register all controls through the SDK before painting their existing
    // custom visuals over the standard button background.
    for control in ui::widgets::take() {
        let _ = draw.widget_button(
            control.rect,
            &Text::literal(control.label),
            control.active,
            false,
        );
    }
    for op in batch.paint {
        match op {
            ui::Paint::Fill(r, c) => draw.fill(r, c),
            ui::Paint::Quad(p, c) => draw.quad(p, c),
            ui::Paint::Text(s, p, c, shadow) => draw.text(&s, p, c, shadow),
            ui::Paint::Sprite(s, src, dst) => {
                draw.sprite(&s, src, dst, None);
            }
            ui::Paint::NineSlice(s, r, c) => draw.gui_nine_slice(&s, r, c),
            ui::Paint::Tooltip(lines, p) => {
                let _ = draw.gui_tooltip(&lines, p);
            }
            ui::Paint::Face(uuid, rect) => {
                if let Some(player) = host.player(uuid) {
                    let _ = draw.push_clip(rect);
                    let view = mod_api::GuiEntityView::new(
                        rect,
                        rect[2] * 2.0,
                        rect[3] * 1.2,
                        mod_api::GuiEntityLook::Turned {
                            yaw: 180.,
                            pitch: 0.,
                        },
                    );
                    let _ = draw.entity(&mod_api::GuiEntity::Existing(player.entity()), view);
                    let _ = draw.pop_clip();
                } else {
                    draw.sprite(
                        ui::tab_list::default_skin(uuid),
                        [8., 8., 8., 8.],
                        rect,
                        None,
                    );
                }
            }
        }
    }
    if let Some((json, time)) = &*overlay().lock().unwrap()
        && time.elapsed().as_secs_f32() < 3.
        && let Ok(Some(text)) = host.text_from_json(json)
    {
        let width = host.text_width(&text).unwrap_or(0);
        let alpha = ((3. - time.elapsed().as_secs_f32()).min(1.) * 255.) as u32;
        let _ = draw.gui_paint(&[mod_api::GuiDraw::Text {
            text,
            x: ((size[0] - width as f32) * 0.5) as i32,
            y: (size[1] - 72.) as i32,
            color: (alpha << 24) | 0xffffff,
            shadow: true,
        }]);
    }
}

fn transparent(id: &str) -> bool {
    id.contains("glass")
        || id.ends_with("_leaves")
        || matches!(
            id,
            "minecraft:air"
                | "minecraft:cave_air"
                | "minecraft:void_air"
                | "minecraft:water"
                | "minecraft:ice"
                | "minecraft:frosted_ice"
        )
}

/// Draw on the entity's own current-frame pose, using the world's depth buffer.
/// The camera's billboard basis removes yaw-dependent width changes and no
/// retained screen coordinate can trail a fast camera turn.
pub(crate) fn player_icon(host: &dyn ClientHost, event: &mut mod_api::RenderPlayerPostEvent<'_>) {
    if event.who.local || event.player.death_time > 0 {
        return;
    }
    let Some(profile) = host
        .client_session()
        .client_tab_list()
        .and_then(|t| t.entries.into_iter().find(|p| p.name == event.who.name))
    else {
        return;
    };
    let Some(texture) = crate::runtime::player_icon_texture(&Bridge::read(host), profile.uuid)
    else {
        return;
    };
    let Some(camera) = host.camera() else { return };
    let Some(entity) = host.entity(event.player.id) else {
        return;
    };
    if entity.is_invisible(host).unwrap_or(false) {
        return;
    }
    let bounds = entity.bounding_box(host);
    let height = bounds.map_or(1.8, |b| (b[1][1] - b[0][1]) as f32);
    let head = event.parts.and_then(|p| p.get(mod_api::HumanoidPart::Head));
    let position = if let Some(head) = head {
        let p = head.pose;
        let pose = glam::Mat4::from_translation(glam::Vec3::new(0., 1.501, 0.))
            * glam::Mat4::from_scale(glam::Vec3::new(-1., -1., 1.))
            * glam::Mat4::from_translation(glam::Vec3::new(p.x, p.y, p.z) / 16.)
            * glam::Mat4::from_quat(glam::Quat::from_euler(
                glam::EulerRot::ZYX,
                p.z_rot,
                p.y_rot,
                p.x_rot,
            ))
            * glam::Mat4::from_scale(glam::Vec3::new(p.x_scale, p.y_scale, p.z_scale));
        let center = pose.transform_point3(glam::Vec3::new(0., -0.25, 0.));
        // A vanilla helmet expands the head by at most 1.5/16 block. Include
        // all eight expanded corners, so pitch cannot move it into the icon.
        let pad = 1.5 / 16.;
        let mut top = f32::NEG_INFINITY;
        for x in [-0.25 - pad, 0.25 + pad] {
            for y in [-0.5 - pad, pad] {
                for z in [-0.25 - pad, 0.25 + pad] {
                    top = top.max(pose.transform_point3(glam::Vec3::new(x, y, z)).y);
                }
            }
        }
        glam::Vec3::new(
            center.x,
            top.max(height) + 0.18 + crate::runtime::HEAD_ICON_SIZE * 0.5,
            center.z,
        )
    } else {
        glam::Vec3::new(0., height + 0.18 + crate::runtime::HEAD_ICON_SIZE * 0.5, 0.)
    };
    let to_local = glam::Mat3::from_rotation_y(event.player.body_yaw.to_radians());
    let right = to_local * glam::Vec3::new(camera.view[0], camera.view[4], camera.view[8]);
    let up = to_local * glam::Vec3::new(camera.view[1], camera.view[5], camera.view[9]);
    let half = crate::runtime::HEAD_ICON_SIZE * 0.5;
    let corners = [
        position - right * half + up * half,
        position - right * half - up * half,
        position + right * half - up * half,
        position + right * half + up * half,
    ]
    .map(|p| p.to_array());
    let light = event.draw.light();
    event.draw.set_light(0x00f000f0);
    event
        .draw
        .quad(&texture, corners, [0., 0., 16., 16.], 0xffffff);
    event.draw.set_light(light);
}

pub(crate) fn flow(cancel: bool) -> mod_api::Flow {
    if cancel {
        mod_api::Flow::Cancel
    } else {
        mod_api::Flow::Continue
    }
}
