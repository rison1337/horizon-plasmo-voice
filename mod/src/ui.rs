//! Private view models keep voice/UI logic independent of the game's ABI.
//! Only sdk.rs translates them to the published mod_api interfaces.
pub use ::mod_api::{Camera as NativeCamera, ScreenView as NativeScreenView, Text};
use std::cell::RefCell;
pub mod hud {
    pub use super::HudBatch;
    pub fn decode_image_bytes(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
        let mut reader = image::ImageReader::new(std::io::Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(128);
        limits.max_image_height = Some(128);
        limits.max_alloc = Some(128 * 128 * 8);
        reader.limits(limits);
        let rgba = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
        Ok((rgba.width(), rgba.height(), rgba.into_raw()))
    }
}
pub mod text {
    pub use ::mod_api::Text;
}
pub mod scoreboard {
    pub fn comp_text(value: &serde_json::Value) -> String {
        value.to_string()
    }
}
#[derive(Clone, Default)]
pub struct NativeClientState {
    pub window_id: u64,
    pub focused: bool,
    pub gui_hidden: bool,
    pub gui_size: [f32; 2],
    pub server_address: Option<String>,
    pub logged_in: bool,
    pub world_partial_tick: f64,
    pub eye_position: Option<[f64; 3]>,
    pub yaw: f32,
    pub pitch: f32,
}
#[derive(Clone, Default)]
pub struct NativeClientEntity {
    pub id: u32,
    pub player_uuid: Option<u128>,
    pub player_name: Option<String>,
    pub position: [f64; 3],
    pub previous_position: [f64; 3],
    pub bounds: [[f64; 3]; 2],
    pub height: f32,
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub alive: bool,
    pub invisible: bool,
}
#[derive(Clone, Default)]
pub struct NativeClientHead {
    pub id: u32,
    pub origin: [f64; 3],
    pub transform: Option<[f32; 16]>,
    pub outer_layer_padding: f32,
    pub height: f32,
    pub name_anchor: [f64; 3],
    pub name_half_width: f32,
}
pub struct NativeClientProfile {
    pub uuid: u128,
    pub skin: String,
}
pub struct NativeHit {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
pub struct Player {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

pub enum Paint {
    Fill([f32; 4], u32),
    Quad([[f32; 2]; 4], u32),
    Text(String, [f32; 2], u32, bool),
    Sprite(String, [f32; 4], [f32; 4]),
    NineSlice(String, [f32; 4], u32),
    Tooltip(Vec<Text>, [f32; 2]),
    Face(u128, [f32; 4]),
}
pub struct HudBatch {
    pub scale: f32,
    pub paint: Vec<Paint>,
}
impl HudBatch {
    pub fn new(_size: [u32; 2]) -> Self {
        Self {
            scale: 1.,
            paint: Vec::new(),
        }
    }
}
pub trait Host {
    fn world_voice_icons(&self) -> bool {
        false
    }
    fn window(&self) -> bool {
        true
    }
    fn on(&self, _: &str) -> bool {
        true
    }
    fn playing(&self) -> bool {
        false
    }
    fn client_state(&self) -> Option<NativeClientState> {
        None
    }
    fn camera(&self) -> Option<NativeCamera> {
        None
    }
    fn player(&self) -> Option<Player> {
        None
    }
    fn client_entities(&self) -> Vec<NativeClientEntity> {
        Vec::new()
    }
    fn client_heads(&self, _: &[u32], _: f64) -> Vec<NativeClientHead> {
        Vec::new()
    }
    fn client_profiles(&self) -> Vec<NativeClientProfile> {
        Vec::new()
    }
    fn client_block_obstruction(&self, _: [i32; 3]) -> Option<(bool, bool)> {
        None
    }
    fn client_visual_clear(&self, _: [f64; 3], _: [f64; 3]) -> bool {
        false
    }
    fn raycast(&self, _: f32) -> Option<NativeHit> {
        None
    }
    fn setting(&self, _: &str, _: &str) -> Option<String> {
        None
    }
    fn set_setting(&self, _: &str, _: &str, _: &str) {}
    fn mod_key(&self, _: &str, _: &str, default: &str) -> String {
        default.into()
    }
    fn set_mod_key(&mut self, _: &str, _: &str, _: Option<&str>) {}
    fn mod_screen(&self) -> Option<&str> {
        None
    }
    fn mod_screen_open(&mut self, _: &str) -> bool {
        false
    }
    fn mod_screen_close(&mut self, _: &str) {}
    fn mod_image(&mut self, _: u32, _: u32, _: &[u8]) -> bool {
        false
    }
    fn send_packet(&mut self, _: crate::network::packets::play::Serverbound) -> Result<(), String> {
        Err("no client connection".into())
    }
    fn client_action_bar(&mut self, _: String, _: bool) {}
    fn text_width(&self, text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }
    fn fill(&self, b: &mut HudBatch, r: [f32; 4], c: u32) {
        b.paint.push(Paint::Fill(r, argb(c)));
    }
    fn quad(&self, b: &mut HudBatch, p: [[f32; 2]; 4], c: u32) {
        b.paint.push(Paint::Quad(p, argb(c)));
    }
    fn text(&self, b: &mut HudBatch, t: &str, p: [f32; 2], c: u32, s: bool) {
        b.paint.push(Paint::Text(t.into(), p, argb(c), s));
    }
    fn sprite(&self, b: &mut HudBatch, n: &str, src: [f32; 4], dst: [f32; 4]) {
        b.paint.push(Paint::Sprite(n.into(), src, dst));
    }
    fn gui_nine_slice(&self, b: &mut HudBatch, n: &str, r: [f32; 4], c: u32) {
        b.paint.push(Paint::NineSlice(n.into(), r, c));
    }
    fn gui_tooltip(&self, b: &mut HudBatch, t: &[Text], p: [f32; 2]) {
        b.paint.push(Paint::Tooltip(t.to_vec(), p));
    }
}

fn argb(color: u32) -> u32 {
    if color <= 0x00ffffff {
        color | 0xff000000
    } else {
        color
    }
}

pub mod lang {
    use super::RefCell;
    thread_local! { static LANGUAGE:RefCell<String> = RefCell::new("en_us".into()); }
    pub fn set(code: String) {
        LANGUAGE.with(|v| *v.borrow_mut() = code);
    }
    pub fn code() -> String {
        LANGUAGE.with(|v| v.borrow().clone())
    }
    pub fn get_or(_key: &str, fallback: &str) -> String {
        fallback.into()
    }
}
pub mod skins {
    use base64::Engine;
    pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
        base64::engine::general_purpose::STANDARD.decode(s).ok()
    }
}
pub mod tab_list {
    pub fn default_skin(_uuid: u128) -> &'static str {
        "minecraft:entity/player/wide/steve"
    }
}
pub mod widgets {
    use super::RefCell;
    pub enum Kind {
        Field,
        Slider { value: f32, track: [f32; 2] },
    }
    pub struct Control {
        pub label: String,
        pub rect: [f32; 4],
        pub active: bool,
    }
    thread_local! { static CONTROLS:RefCell<Vec<Control>>=const { RefCell::new(Vec::new()) }; }
    pub fn record(label: &str, rect: [f32; 4], active: bool) {
        CONTROLS.with(|c| {
            c.borrow_mut().push(Control {
                label: label.into(),
                rect,
                active,
            })
        });
    }
    pub fn record_as(label: &str, rect: [f32; 4], active: bool, kind: Kind) {
        if let Kind::Slider { value, track } = kind {
            let _ = (value, track);
        }
        record(label, rect, active);
    }
    pub fn take() -> Vec<Control> {
        CONTROLS.with(|v| std::mem::take(&mut *v.borrow_mut()))
    }
}
