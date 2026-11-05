//! The voice mod's public-API adapter. No Game modules, private fields or raw targets.
use horizon_client::mod_api::*;
mod menu;
mod runtime;
mod voice_art;

trait HostExt: Host {
    fn gui_size(&self) -> (f32, f32) {
        let size = self.client_state().map_or([0.0, 0.0], |v| v.gui_size);
        (size[0], size[1])
    }
}
impl<T: Host + ?Sized> HostExt for T {}

struct Voice;
/// Called by the launcher's ordinary `[api]` entry point.
pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Voice)
}
impl Hooks for Voice {
    fn client_disconnecting(&self, host: &dyn Host, _: &mut ClientDisconnectingEvent) {
        runtime::disconnect(host);
    }
    fn client_stopping(&self, host: &dyn Host, _: &mut ClientStoppingEvent) {
        runtime::disconnect(host);
    }
    fn frame(&self, host: &mut dyn Host, _: &mut FrameEvent) {
        runtime::frame(host);
    }
    fn client_packet(&self, host: &mut dyn Host, event: &mut ClientPacketEvent<'_>) {
        if let ClientboundPlayPacket::CustomPayload(p) = event.packet
            && p.channel == "plasmo:voice/v2"
        {
            runtime::payload(host, &p.data);
        }
    }
    fn key(&self, host: &mut dyn Host, e: &mut KeyEvent<'_>) {
        runtime::key(host, e.key, e.pressed);
    }
    fn input_key(&self, host: &mut dyn Host, e: &mut InputKeyEvent<'_>) {
        runtime::raw_key(host, e.event);
    }
    fn mouse_button_pre(&self, host: &mut dyn Host, e: &mut MouseButtonPreEvent) -> Flow {
        runtime::raw_mouse(host, e.button, e.pressed);
        Flow::Continue
    }
    fn wheel(&self, host: &mut dyn Host, e: &mut WheelEvent) -> Flow {
        let mut claimed = false;
        runtime::wheel(host, e.steps, &mut claimed);
        if claimed {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
    fn mods(&self, _: &mut dyn Host, e: &mut ModsEvent<'_>) {
        runtime::mods(e.mods);
    }
    fn settings(&self, host: &mut dyn Host, e: &mut SettingsEvent<'_>) {
        if e.mod_id == runtime::ID {
            runtime::settings(host, e.buttons);
        }
    }
    fn settings_click(&self, host: &mut dyn Host, e: &mut SettingsClickEvent<'_>) {
        if e.mod_id == runtime::ID {
            runtime::settings_click(host, e.button);
        }
    }
    fn hud(&self, host: &mut dyn Host, e: &mut HudEvent<'_>) {
        runtime::hud(host, e.batch);
    }
    fn gui_render_post(&self, host: &dyn Host, e: &mut GuiRenderPostEvent<'_>) {
        runtime::foreground_hud(host, e.batch);
    }
    fn menu_background(&self, host: &dyn Host, e: &mut MenuBackgroundEvent<'_>) {
        if host.mod_screen() == Some(runtime::ID)
            && let Some(view) = host.client_state()
        {
            *e.radius = (view.menu_blur >= 1.0).then_some(view.menu_blur);
        }
    }
    fn screen_render_post(&self, host: &dyn Host, e: &mut ScreenRenderPostEvent<'_>) {
        runtime::render(host, e.batch, e.screen);
    }
    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn Host,
        e: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        let mut cancel = false;
        runtime::mouse(host, e.screen, e.button, true, &mut cancel);
        if cancel { Flow::Cancel } else { Flow::Continue }
    }
    fn screen_mouse_button_released_pre(
        &self,
        host: &dyn Host,
        e: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        let mut cancel = false;
        runtime::mouse(host, e.screen, e.button, false, &mut cancel);
        if cancel { Flow::Cancel } else { Flow::Continue }
    }
    fn screen_mouse_dragged_pre(
        &self,
        host: &dyn Host,
        e: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        let mut cancel = false;
        runtime::drag(host, e.screen, &mut cancel);
        if cancel { Flow::Cancel } else { Flow::Continue }
    }
    fn screen_mouse_scrolled_pre(
        &self,
        host: &dyn Host,
        e: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        let mut cancel = false;
        runtime::scroll(host, e.scroll[1], &mut cancel);
        if cancel { Flow::Cancel } else { Flow::Continue }
    }
    fn screen_key_pressed_pre(
        &self,
        host: &dyn Host,
        e: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        let mut cancel = false;
        runtime::screen_key(host, e.key, e.repeat, &mut cancel);
        if cancel { Flow::Cancel } else { Flow::Continue }
    }
    fn screen_key_released_pre(
        &self,
        host: &dyn Host,
        e: &mut ScreenKeyReleasedPreEvent<'_>,
    ) -> Flow {
        runtime::screen_key_release(host, e.key);
        Flow::Continue
    }
    fn screen_char_typed_pre(&self, host: &dyn Host, e: &mut ScreenCharTypedPreEvent<'_>) -> Flow {
        let mut cancel = false;
        runtime::character(host, e.c, &mut cancel);
        if cancel { Flow::Cancel } else { Flow::Continue }
    }
}

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
