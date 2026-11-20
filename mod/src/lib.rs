//! Plasmo Voice through the exact published Horizon SDK.
#[path = "voice/lib.rs"]
mod voice_engine;
pub(crate) use voice_engine::runtime_cache;
pub use voice_engine::*;
mod menu;
mod network;
mod runtime;
mod sdk;
mod strings;
mod ui;
mod voice_art;

trait HostExt: ui::Host {
    fn gui_size(&self) -> (f32, f32) {
        let size = self.client_state().map_or([0.0, 0.0], |v| v.gui_size);
        (size[0], size[1])
    }
}
impl<T: ui::Host + ?Sized> HostExt for T {}

fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let (yaw, pitch) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        yaw.cos() * pitch.cos(),
    ]
}
fn ray_box(eye: [f64; 3], direction: [f64; 3], reach: f64, bounds: [[f64; 3]; 2]) -> Option<f64> {
    let (mut near, mut far) = (0.0_f64, reach);
    for axis in 0..3 {
        if direction[axis].abs() < 1e-9 {
            if eye[axis] < bounds[0][axis] || eye[axis] > bounds[1][axis] {
                return None;
            }
        } else {
            let a = (bounds[0][axis] - eye[axis]) / direction[axis];
            let b = (bounds[1][axis] - eye[axis]) / direction[axis];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
        }
    }
    (near <= far).then_some(near)
}

use mod_api::{ClientHost, Hooks};
use sdk::{Bridge, flow, paint};
#[derive(Default)]
struct Voice;
#[mod_api::horizon_mod]
impl Hooks for Voice {
    fn render_player_post(&self, h: &dyn ClientHost, e: &mut mod_api::RenderPlayerPostEvent<'_>) {
        sdk::player_icon(h, e);
    }
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        for (id, key, label) in [
            ("menu", "key.keyboard.v", "open_menu"),
            ("talk", "key.keyboard.left.alt", "talk"),
            ("mute", "key.keyboard.m", "mute"),
            ("deafen", "", "disable_voice"),
            ("action", "key.mouse.right", "action_key"),
            ("occlusion", "", "toggle_occlusion"),
        ] {
            r.register_key_mapping(
                mod_api::KeyRegistration::new(id, key).label(&format!("plasmo_voice.{label}")),
            );
        }
        r.payload_handlers()
            .registrar("1")
            .play_raw(mod_api::PayloadId("plasmo:voice/v2".into()), None);
    }
    fn render_frame_pre(&self, h: &mut dyn ClientHost, _: &mut mod_api::RenderFramePreEvent) {
        runtime::frame(&mut Bridge::write(h));
    }
    fn client_disconnecting(&self, h: &dyn ClientHost, _: &mut mod_api::ClientDisconnectingEvent) {
        runtime::disconnect(&Bridge::read(h));
    }
    fn client_stopping(&self, h: &mut dyn ClientHost, _: &mut mod_api::ClientStoppingEvent) {
        audio_runtime::shutdown();
        runtime::disconnect(&Bridge::read(h));
    }
    fn client_packet(&self, h: &mut dyn ClientHost, e: &mut mod_api::ClientPacketEvent<'_>) {
        if let Some(p) = &e.packet.custom_payload
            && p.channel == "plasmo:voice/v2"
        {
            runtime::payload(&mut Bridge::write(h), &p.data);
        }
    }
    fn input_key(&self, h: &mut dyn ClientHost, e: &mut mod_api::InputKeyEvent<'_>) {
        match e.action {
            mod_api::KeyAction::Press => runtime::key(&mut Bridge::write(h), e.key, true),
            mod_api::KeyAction::Release => runtime::key(&mut Bridge::write(h), e.key, false),
            _ => {}
        }
    }
    fn mouse_button_pre(
        &self,
        h: &mut dyn ClientHost,
        e: &mut mod_api::MouseButtonPreEvent,
    ) -> mod_api::Flow {
        runtime::raw_mouse(&mut Bridge::write(h), e.button, e.pressed);
        mod_api::Flow::Continue
    }
    fn mouse_scrolling(
        &self,
        h: &mut dyn ClientHost,
        e: &mut mod_api::MouseScrollingEvent,
    ) -> mod_api::Flow {
        let mut cancel = false;
        runtime::wheel(&mut Bridge::write(h), e.accumulated[1] as f32, &mut cancel);
        flow(cancel)
    }
    fn gui_render_post(&self, h: &dyn ClientHost, e: &mut mod_api::GuiRenderPostEvent<'_>) {
        paint(h, e.batch, |bridge, b| {
            runtime::hud(bridge, b);
            runtime::foreground_hud(bridge, b);
        });
    }
    fn menu_background(&self, h: &dyn ClientHost, e: &mut mod_api::MenuBackgroundEvent<'_>) {
        if h.mod_screen() == Some(runtime::ID) {
            let blur = h.gui_options().menu_blur as f32;
            *e.radius = (blur >= 1.).then_some(blur);
        }
    }
    fn screen_render_post(&self, h: &dyn ClientHost, e: &mut mod_api::ScreenRenderPostEvent<'_>) {
        paint(h, e.draw, |bridge, b| runtime::render(bridge, b, e.screen));
    }
    fn screen_mouse_button_pressed_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenMouseButtonPressedPreEvent<'_>,
    ) -> mod_api::Flow {
        let mut c = false;
        runtime::mouse(&Bridge::read(h), e.screen, e.button, true, &mut c);
        flow(c)
    }
    fn screen_mouse_button_released_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> mod_api::Flow {
        let mut c = false;
        runtime::mouse(&Bridge::read(h), e.screen, e.button, false, &mut c);
        flow(c)
    }
    fn screen_mouse_dragged_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenMouseDraggedPreEvent<'_>,
    ) -> mod_api::Flow {
        let mut c = false;
        runtime::drag(&Bridge::read(h), e.screen, &mut c);
        flow(c)
    }
    fn screen_mouse_scrolled_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenMouseScrolledPreEvent<'_>,
    ) -> mod_api::Flow {
        let mut c = false;
        runtime::scroll(&Bridge::read(h), e.scroll[1], &mut c);
        flow(c)
    }
    fn screen_key_pressed_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenKeyPressedPreEvent<'_>,
    ) -> mod_api::Flow {
        let mut c = false;
        runtime::screen_key(&Bridge::read(h), e.key, e.repeat, &mut c);
        flow(c)
    }
    fn screen_key_released_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenKeyReleasedPreEvent<'_>,
    ) -> mod_api::Flow {
        runtime::screen_key_release(&Bridge::read(h), e.key);
        mod_api::Flow::Continue
    }
    fn screen_char_typed_pre(
        &self,
        h: &dyn ClientHost,
        e: &mut mod_api::ScreenCharTypedPreEvent<'_>,
    ) -> mod_api::Flow {
        let mut c = false;
        runtime::character(&Bridge::read(h), e.c, &mut c);
        flow(c)
    }
}
