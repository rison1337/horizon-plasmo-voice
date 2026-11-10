// Native recreation from black-box observations of released Plasmo Voice 2.1.17.
// GUI primitives keep their coordinates, style and action arguments explicit.
#![allow(clippy::too_many_arguments)]
use super::{
    HostExt,
    runtime::{ID, State, tr},
    voice_art::{self as art, Glyph, Mark},
};
use horizon_client::mod_api::Host;
use horizon_client::{
    hud::HudBatch,
    mod_api::NativeScreenView,
    widgets::{self, Kind},
};
use horizon_plasmo_engine::{
    activation::Mode,
    protocol::Activation,
    session::{Anchor, IconVisibility, OverlayMode, OverlayStyle, SourceOverlap, Status},
    settings_flags::Flag,
};

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub(super) enum Tab {
    #[default]
    Devices,
    Volume,
    Activation,
    Overlay,
    Advanced,
    Hotkeys,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Slider {
    Gain,
    Volume,
    Threshold,
    Angle,
    Line(u128),
    Player(u128),
    Distance(u128, u16, u16),
}
impl Slider {
    pub(super) fn range(self) -> (f32, f32) {
        match self {
            Self::Gain => (0., 2.),
            Self::Threshold => (-60., 0.),
            Self::Angle => (90., 360.),
            Self::Distance(_, min, max) => (f32::from(min), f32::from(max)),
            _ => (0., 2.),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Choice {
    Input,
    Output,
    Mode(u128),
    Icons,
    Style,
    Overlap,
    LineOverlay(u128),
}
#[derive(Clone)]
pub(super) enum Action {
    Tab(Tab),
    Open(Choice),
    Choose(Choice, usize),
    Search,
    Close,
    Preview,
    Muted,
    Enabled,
    Flag(Flag, Option<bool>),
    Position(bool),
    Anchor(bool, Anchor),
    LineMute(u128),
    PlayerMute(u128),
    Value(Slider, f32),
    Bind(String),
    Bound(String, String),
    ResetKey(String),
    Input(String),
    Output(String),
}
#[derive(Clone, Copy)]
struct Rect([f32; 4]);
impl Rect {
    fn has(self, p: [f32; 2]) -> bool {
        let [x, y, w, h] = self.0;
        p[0] >= x && p[0] < x + w && p[1] >= y && p[1] < y + h
    }
}
#[derive(Clone)]
struct Widget {
    rect: Rect,
    action: Action,
    slider: Option<Slider>,
}
#[derive(Default)]
struct Tabs {
    layout: Option<(f32, f32, String)>,
    rects: Vec<[f32; 4]>,
    labels: Vec<String>,
    content_top: f32,
}
impl Tabs {
    fn update(&mut self, g: &dyn Host, width: f32, height: f32) {
        let key = (width, height, horizon_client::lang::code());
        if self.layout.as_ref() == Some(&key) {
            return;
        }
        self.layout = Some(key);
        self.rects.clear();
        self.labels.clear();
        let (mut x, mut y) = (14., 36.);
        for key in [
            "devices",
            "volumes",
            "activation",
            "overlay",
            "advanced",
            "hotkeys",
        ] {
            let label = tr(key);
            let w = g.text_width(&label).ceil() + 24.;
            if x + w > width - 14. && x > 14. {
                x = 14.;
                y += 26.;
            }
            self.rects.push([x, y, w, 20.]);
            self.labels.push(label);
            x += w + 4.;
        }
        self.content_top = y + 33.;
    }
}
#[derive(Default)]
pub(super) struct Menu {
    tabs: Tabs,
    tab: Tab,
    scroll: f32,
    max_scroll: f32,
    tooltip: Option<String>,
    widgets: Vec<Widget>,
    drag: Option<(Slider, Rect)>,
    dropdown: Option<Choice>,
    drop_scroll: usize,
    search: String,
    searching: bool,
    pub(super) binding: Option<String>,
    binding_modifiers: Vec<String>,
    pub(super) position: Option<bool>,
    pub(super) pending: Vec<Action>,
}
impl Menu {
    pub(super) fn devices_tab(&self) -> bool {
        self.tab == Tab::Devices && self.position.is_none()
    }
    pub(super) fn dragging(&self) -> bool {
        self.drag.is_some()
    }
    fn queue(&mut self, action: Action) {
        match action {
            Action::Tab(tab) => {
                self.tab = tab;
                self.scroll = 0.;
                self.dropdown = None;
                self.searching = tab == Tab::Volume;
            }
            Action::Open(choice) => {
                self.dropdown = if self.dropdown == Some(choice) {
                    None
                } else {
                    Some(choice)
                };
                self.drop_scroll = 0;
            }
            Action::Search => self.searching = true,
            Action::Bind(name) => {
                self.binding = Some(name);
                self.binding_modifiers.clear();
                self.dropdown = None;
            }
            Action::Position(hud) => {
                self.position = Some(hud);
                self.dropdown = None;
            }
            Action::Choose(..) | Action::Input(..) | Action::Output(..) => {
                self.dropdown = None;
                self.pending.push(action);
            }
            Action::Anchor(..) => {
                self.position = None;
                self.pending.push(action);
            }
            other => {
                if self.pending.len() < 32 {
                    self.pending.push(other);
                }
            }
        }
    }
    pub(super) fn mouse(&mut self, point: [f32; 2], pressed: bool) {
        if !pressed {
            self.drag = None;
            return;
        }
        if let Some(binding) = self.binding.take() {
            self.binding = Some(binding);
            self.bind_key("key.mouse.left");
            return;
        }
        let hit = self
            .widgets
            .iter()
            .rev()
            .find(|w| w.rect.has(point))
            .cloned();
        if let Some(w) = hit {
            if let Some(slider) = w.slider {
                self.drag = Some((slider, w.rect));
                self.dragged(point);
            } else {
                self.queue(w.action);
            }
        } else {
            self.dropdown = None;
            self.searching = false;
        }
    }
    pub(super) fn dragged(&mut self, point: [f32; 2]) {
        if let Some((slider, rect)) = self.drag {
            let (min, max) = slider.range();
            let [x, _, w, _] = rect.0;
            let value = min + ((point[0] - x - 4.) / (w - 8.)).clamp(0., 1.) * (max - min);
            if matches!(self.pending.last(),Some(Action::Value(previous,_)) if *previous==slider) {
                self.pending.pop();
            }
            self.pending.push(Action::Value(slider, value));
        }
    }
    pub(super) fn wheel(&mut self, steps: f64) {
        if self.dropdown.is_some() {
            self.drop_scroll = (self.drop_scroll as i32 - (steps * 2.) as i32).max(0) as usize;
        } else {
            self.scroll = (self.scroll - steps as f32 * 24.).clamp(0., self.max_scroll);
        }
    }
    pub(super) fn key_release(&mut self, key: &str) {
        if self.binding.is_some() && self.binding_modifiers.iter().any(|v| v == key) {
            self.bind_key("");
        }
    }
    pub(super) fn bind_key(&mut self, key: &str) {
        if let Some(binding) = self.binding.take() {
            let mut keys = std::mem::take(&mut self.binding_modifiers);
            if !key.is_empty() {
                keys.push(key.into());
            }
            self.pending.push(Action::Bound(binding, keys.join("+")));
        }
    }
    pub(super) fn key(&mut self, key: &str, menu_key: &str) {
        if self.binding.is_some()
            && ["shift", "control", "alt", "super"]
                .iter()
                .any(|m| key.ends_with(m))
        {
            if !self.binding_modifiers.iter().any(|v| v == key) {
                self.binding_modifiers.push(key.into());
            }
            return;
        }
        if self.binding.is_some() {
            if key != "key.keyboard.escape" {
                self.bind_key(key);
            } else {
                self.binding_modifiers.clear();
                self.bind_key("");
            }
        } else if key == "key.keyboard.escape" {
            if self.position.take().is_none() && self.dropdown.take().is_none() {
                self.queue(Action::Close);
            }
        } else if !self.searching && key == menu_key {
            self.queue(Action::Close);
        } else if self.searching && key == "key.keyboard.backspace" {
            self.search.pop();
        }
    }
    pub(super) fn character(&mut self, c: char) {
        if self.searching && !c.is_control() && self.search.chars().count() < 32 {
            self.search.push(c);
        }
    }
    pub(super) fn reset(&mut self) {
        self.drag = None;
        self.dropdown = None;
        self.binding = None;
        self.position = None;
        self.searching = false;
    }
}
const WHITE: u32 = 0xffffff;
const DIM: u32 = 0xa0a0a0;
const GREEN: u32 = 0x55ff55;
const RED: u32 = 0xff5555;
fn text(g: &dyn Host, b: &mut HudBatch, s: &str, x: f32, y: f32, c: u32) {
    g.text(b, s, [x, y], c, true);
}
fn center(g: &dyn Host, b: &mut HudBatch, s: &str, x: f32, y: f32, w: f32, c: u32) {
    text(g, b, s, x + (w - g.text_width(s)) * 0.5, y, c);
}
fn trim(g: &dyn Host, s: &str, w: f32) -> String {
    let mut s = s.to_owned();
    if g.text_width(&s) <= w {
        return s;
    }
    while !s.is_empty() && g.text_width(&(s.clone() + "...")) > w {
        s.pop();
    }
    s + "..."
}
fn fill(g: &dyn Host, b: &mut HudBatch, r: [f32; 4], c: u32) {
    g.fill(b, r, c);
}
fn panel(g: &dyn Host, b: &mut HudBatch, r: [f32; 4], hover: bool, disabled: bool) {
    let name = if disabled {
        "button_disabled"
    } else if hover {
        "button_highlighted"
    } else {
        "button"
    };
    g.gui_nine_slice(b, &format!("gui/sprites/widget/{name}"), r, u32::MAX);
}
fn button(
    g: &dyn Host,
    b: &mut HudBatch,
    m: &mut Menu,
    mouse: [f32; 2],
    r: [f32; 4],
    label: &str,
    a: Action,
    disabled: bool,
) {
    panel(g, b, r, Rect(r).has(mouse), disabled);
    center(
        g,
        b,
        &trim(g, label, r[2] - 6.),
        r[0],
        r[1] + 6.,
        r[2],
        if disabled { DIM } else { WHITE },
    );
    widgets::record(label, r, !disabled);
    if !disabled {
        m.widgets.push(Widget {
            rect: Rect(r),
            action: a,
            slider: None,
        });
    }
}
fn track(g: &dyn Host, b: &mut HudBatch, r: [f32; 4], value: f32, hover: bool) {
    let [x, y, w, h] = r;
    g.gui_nine_slice(b, "gui/sprites/widget/slider", r, u32::MAX);
    let name = if hover {
        "slider_handle_highlighted"
    } else {
        "slider_handle"
    };
    g.gui_nine_slice(
        b,
        &format!("gui/sprites/widget/{name}"),
        [x + (w - 8.0) * value, y, 8.0, h],
        u32::MAX,
    );
}
fn reset_icon(g: &dyn Host, b: &mut HudBatch, x: f32, y: f32, c: u32) {
    pixel_icon(g, b, Glyph::Reset, Mark::None, x, y, 0.75, c);
}

fn small_icon(g: &dyn Host, b: &mut HudBatch, kind: usize, x: f32, y: f32, c: u32) {
    match kind {
        0 => headphones(g, b, x, y, 0.5, c, false),
        1 => {
            for i in 0..3 {
                fill(
                    g,
                    b,
                    [
                        x + i as f32 * 2.,
                        y + 5. - i as f32 * 2.,
                        1.,
                        2. + i as f32 * 2.,
                    ],
                    0xff55aa55,
                );
            }
        }
        2 => microphone(g, b, x, y, 0.5, c, false),
        3 => {
            fill(g, b, [x, y + 1., 8., 5.], 0xff000000 | c);
            fill(g, b, [x + 1., y + 2., 6., 3.], 0xff555555);
            fill(g, b, [x + 3., y + 6., 2., 2.], 0xff000000 | c);
        }
        4 => {
            for r in [
                [2., 0., 4., 1.],
                [0., 2., 1., 4.],
                [7., 2., 1., 4.],
                [2., 7., 4., 1.],
                [1., 1., 2., 1.],
                [5., 1., 2., 1.],
                [1., 6., 2., 1.],
                [5., 6., 2., 1.],
                [2., 2., 4., 1.],
                [2., 5., 4., 1.],
                [2., 3., 1., 2.],
                [5., 3., 1., 2.],
            ] {
                fill(g, b, [x + r[0], y + r[1], r[2], r[3]], 0xff000000 | c);
            }
        }
        _ => {
            fill(g, b, [x, y + 3., 8., 5.], 0xff000000 | c);
            for i in 0..3 {
                fill(g, b, [x + 1. + i as f32 * 2., y + 4., 1., 1.], 0xff555555);
            }
        }
    }
}
struct Form<'a> {
    g: &'a dyn Host,
    b: &'a mut HudBatch,
    m: &'a mut Menu,
    mouse: [f32; 2],
    x: f32,
    w: f32,
    y: f32,
    top: f32,
    bottom: f32,
}
impl Form<'_> {
    fn shown(&self) -> bool {
        self.y >= self.top && self.y + 20. <= self.bottom
    }
    fn title(&mut self, key: &str) {
        self.title_text(&tr(key));
    }
    fn title_text(&mut self, label: &str) {
        if self.y >= self.top && self.y + 14. < self.bottom {
            center(self.g, self.b, label, self.x, self.y + 3., self.w, WHITE);
        }
        self.y += 21.;
    }
    fn rect(&self) -> [f32; 4] {
        [self.x + self.w * 0.5 + 4., self.y, self.w * 0.5 - 28., 20.]
    }
    fn label(&mut self, label: &str) {
        if Rect([self.x, self.y, self.w * 0.5, 20.]).has(self.mouse) {
            for key in [
                "noise_suppression",
                "stereo_capture",
                "disable_input",
                "sound_occlusion",
                "directional_sources",
                "hrtf",
                "mono_sources",
                "directional_angle",
                "adaptive_jitter",
                "panning",
                "source_overlap",
                "visualize_distance",
                "visualize_join",
                "volume_slider",
                "distance_gain",
            ] {
                if label == tr(key) {
                    let hint = tr(&format!("hint_{key}"));
                    if hint != format!("hint_{key}") {
                        self.m.tooltip = Some(hint);
                    }
                    break;
                }
            }
        }
        text(
            self.g,
            self.b,
            &trim(self.g, label, self.w * 0.5 - 7.),
            self.x + 1.,
            self.y + 6.,
            WHITE,
        );
    }
    fn reset(&mut self, action: Action, enabled: bool) {
        let r = [self.x + self.w - 20., self.y, 20., 20.];
        panel(self.g, self.b, r, Rect(r).has(self.mouse), !enabled);
        reset_icon(
            self.g,
            self.b,
            r[0] + 4.,
            r[1] + 4.,
            if enabled { WHITE } else { 0xa0a0a0 },
        );
        widgets::record(&tr("reset"), r, enabled);
        if enabled {
            self.m.widgets.push(Widget {
                rect: Rect(r),
                action,
                slider: None,
            });
        }
    }
    fn row(&mut self, label: &str, value: &str, action: Action, reset: Action, changed: bool) {
        if self.shown() {
            self.label(label);
            let r = self.rect();
            button(self.g, self.b, self.m, self.mouse, r, value, action, false);
            self.reset(reset, changed);
        }
        self.y += 24.;
    }
    fn toggle(&mut self, label: &str, flag: Flag, s: &horizon_plasmo_engine::session::Settings) {
        if self.shown() {
            self.label(label);
            let r = self.rect();
            let value = flag.get(s);
            track(
                self.g,
                self.b,
                r,
                if value { 1. } else { 0. },
                Rect(r).has(self.mouse),
            );
            center(
                self.g,
                self.b,
                &tr(if value { "on" } else { "off" }),
                r[0],
                r[1] + 6.,
                r[2],
                WHITE,
            );
            widgets::record(
                &format!("{label}: {}", tr(if value { "on" } else { "off" })),
                r,
                true,
            );
            self.m.widgets.push(Widget {
                rect: Rect(r),
                action: Action::Flag(flag, None),
                slider: None,
            });
            self.reset(
                Action::Flag(flag, Some(flag.default_value())),
                value != flag.default_value(),
            );
        }
        self.y += 24.;
    }
    fn combo(&mut self, label: &str, value: &str, choice: Choice, default: usize, changed: bool) {
        if self.shown() {
            self.label(label);
            let r = self.rect();
            let [x, y, w, h] = r;
            fill(self.g, self.b, r, 0xffb0b0b0);
            fill(self.g, self.b, [x + 1., y + 1., w - 2., h - 2.], 0xff000000);
            text(
                self.g,
                self.b,
                &trim(self.g, value, w - 17.),
                x + 4.,
                y + 6.,
                WHITE,
            );
            self.g.quad(
                self.b,
                [
                    [x + w - 12., y + 7.],
                    [x + w - 3., y + 7.],
                    [x + w - 7., y + 11.],
                    [x + w - 12., y + 7.],
                ],
                0xffa0a0a0,
            );
            widgets::record(value, r, true);
            self.m.widgets.push(Widget {
                rect: Rect(r),
                action: Action::Open(choice),
                slider: None,
            });
            self.reset(Action::Choose(choice, default), changed);
        }
        self.y += 24.;
    }
    fn overlay_toggle(&mut self, label: &str, id: u128, enabled: bool) {
        if self.shown() {
            self.label(label);
            let r = self.rect();
            track(
                self.g,
                self.b,
                r,
                if enabled { 1. } else { 0. },
                Rect(r).has(self.mouse),
            );
            center(
                self.g,
                self.b,
                &tr(if enabled { "on" } else { "off" }),
                r[0],
                r[1] + 6.,
                r[2],
                WHITE,
            );
            widgets::record(
                &format!("{label}: {}", tr(if enabled { "on" } else { "off" })),
                r,
                true,
            );
            self.m.widgets.push(Widget {
                rect: Rect(r),
                action: Action::Choose(Choice::LineOverlay(id), usize::from(!enabled)),
                slider: None,
            });
            self.reset(Action::Choose(Choice::LineOverlay(id), 0), enabled);
        }
        self.y += 24.;
    }
    fn slider(&mut self, label: &str, slider: Slider, value: f32, default: f32, unit: &str) {
        if self.shown() {
            self.label(label);
            let r = self.rect();
            self.slider_at(label, slider, value, unit, r);
            self.reset(
                Action::Value(slider, default),
                (value - default).abs() > 0.001,
            );
        }
        self.y += 24.;
    }
    fn volume(
        &mut self,
        label: &str,
        slider: Slider,
        value: f32,
        default: f32,
        muted: bool,
        skin: Option<&str>,
        icon: Option<[f32; 4]>,
    ) {
        if self.shown() {
            if let Some(skin) = skin {
                paint_face(self.g, self.b, skin, self.x + 2., self.y + 4., 12);
            } else if let Some(rect) = icon {
                self.g.sprite(
                    self.b,
                    "horizon/modimage",
                    rect,
                    [self.x + 2., self.y + 2., 16., 16.],
                );
            } else {
                speaker(self.g, self.b, self.x + 5., self.y + 5., 0.55, WHITE, false);
            }
            text(
                self.g,
                self.b,
                &trim(self.g, label, self.w * 0.5 - 26.),
                self.x + 20.,
                self.y + 6.,
                WHITE,
            );
            let mut r = self.rect();
            r[2] -= 24.;
            self.slider_at(label, slider, value, "%", r);
            let r = [r[0] + r[2] + 4., r[1], 20., 20.];
            panel(self.g, self.b, r, Rect(r).has(self.mouse), false);
            speaker(self.g, self.b, r[0] + 3., r[1] + 2., 1., WHITE, muted);
            let action = match slider {
                Slider::Line(id) => Action::LineMute(id),
                Slider::Player(id) => Action::PlayerMute(id),
                _ => return,
            };
            widgets::record(&format!("{}: {label}", tr("mute")), r, true);
            self.m.widgets.push(Widget {
                rect: Rect(r),
                action,
                slider: None,
            });
            self.reset(
                Action::Value(slider, default),
                (value - default).abs() > 0.001,
            );
        }
        self.y += 24.;
    }
    fn slider_at(&mut self, label: &str, slider: Slider, value: f32, unit: &str, r: [f32; 4]) {
        let (min, max) = slider.range();
        let t = ((value - min) / (max - min)).clamp(0., 1.);
        track(self.g, self.b, r, t, Rect(r).has(self.mouse));
        let value_label = if unit == "%" {
            format!("{:.0}%", value * 100.)
        } else if unit.is_empty() {
            format!("{value:.0}")
        } else {
            format!("{value:.0} {unit}")
        };
        center(self.g, self.b, &value_label, r[0], r[1] + 6., r[2], WHITE);
        widgets::record_as(
            &format!("{label}: {value_label}"),
            r,
            true,
            Kind::Slider {
                value: t,
                track: [r[0] + 4., r[2] - 8.],
            },
        );
        self.m.widgets.push(Widget {
            rect: Rect(r),
            action: Action::Value(slider, value),
            slider: Some(slider),
        });
    }
}
fn key_label(key: &str) -> String {
    if key.contains('+') {
        return key
            .split('+')
            .map(key_label)
            .collect::<Vec<_>>()
            .join(" + ");
    }
    match key {
        "" | "key.keyboard.unknown" => tr("unbound"),
        "key.mouse.left" => tr("mouse_left"),
        "key.mouse.right" => tr("mouse_right"),
        "key.keyboard.caps.lock" => "Caps Lock".into(),
        "key.keyboard.left.alt" => tr("left_alt"),
        "key.keyboard.left.shift" => tr("left_shift"),
        "key.keyboard.left.control" => tr("left_control"),
        _ => key
            .strip_prefix("key.keyboard.")
            .unwrap_or(key)
            .replace('.', " ")
            .to_uppercase(),
    }
}
fn server_label(state: &State, key: &str, fallback: &str) -> String {
    state
        .snapshot
        .languages
        .get(&horizon_client::lang::code())
        .or_else(|| state.snapshot.languages.get("en_us"))
        .and_then(|m| m.get(key))
        .cloned()
        .unwrap_or_else(|| fallback.to_owned())
}
fn activation_label(state: &State, a: &Activation) -> String {
    let fallback = if a.name == "proximity" {
        tr("proximity")
    } else {
        a.name.clone()
    };
    server_label(state, &a.translation, &fallback)
}
fn mode_label(mode: Mode) -> String {
    tr(match mode {
        Mode::PushToTalk => "ptt",
        Mode::Voice => "voice_activation",
        Mode::Inherit => "inherit",
    })
}
fn anchor_label(anchor: Anchor) -> String {
    tr(match anchor {
        Anchor::TopLeft => "top_left",
        Anchor::TopCenter => "top_center",
        Anchor::TopRight => "top_right",
        Anchor::BottomLeft => "bottom_left",
        Anchor::BottomCenter => "bottom_center",
        Anchor::BottomRight => "bottom_right",
        _ => "center",
    })
}
pub(super) fn render(
    g: &dyn Host,
    state: &mut State,
    b: &mut HudBatch,
    screen: &NativeScreenView<'_>,
) {
    let [_, _, width, height] = screen.rect;
    let mouse = screen.mouse;
    state.ui.widgets.clear();
    state.ui.tooltip = None;
    state.ui.tabs.update(g, width, height);
    let header_bottom = state.ui.tabs.content_top - 4.;
    fill(g, b, [0., 0., width, header_bottom], 0x66000000);
    if let Some(message) = super::runtime::unavailable_message() {
        text(g, b, &tr("settings_title"), 14., 16., WHITE);
        let mut line = String::new();
        let mut y = header_bottom + 18.;
        for word in message.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if g.text_width(&candidate) > (width - 32.).max(32.) && !line.is_empty() {
                text(g, b, &line, 16., y, WHITE);
                y += 12.;
                line = word.into();
            } else {
                line = candidate;
            }
        }
        text(g, b, &line, 16., y, WHITE);
        return;
    }
    if let Some(hud) = state.ui.position {
        center(
            g,
            b,
            &tr("choose_position"),
            0.,
            height * 0.5 - 8.,
            width,
            WHITE,
        );
        for (anchor, col, row) in [
            (Anchor::TopLeft, 0, 0),
            (Anchor::TopCenter, 1, 0),
            (Anchor::TopRight, 2, 0),
            (Anchor::BottomLeft, 0, 1),
            (Anchor::BottomCenter, 1, 1),
            (Anchor::BottomRight, 2, 1),
        ] {
            let x = match col {
                0 => 25.,
                1 => (width - 100.) * 0.5,
                _ => width - 125.,
            };
            let y = if row == 0 { 25. } else { height - 45. };
            button(
                g,
                b,
                &mut state.ui,
                mouse,
                [x, y, 100., 20.],
                &anchor_label(anchor),
                Action::Anchor(hud, anchor),
                anchor
                    == if hud {
                        state.settings.overlay_position
                    } else {
                        state.settings.hud_position
                    },
            );
        }
        return;
    }
    text(g, b, &tr("settings_title"), 14., 16., WHITE);
    icon_button(
        g,
        b,
        &mut state.ui,
        mouse,
        [width - 51., 9., 20., 20.],
        Action::Muted,
        state.settings.muted,
    );
    icon_button(
        g,
        b,
        &mut state.ui,
        mouse,
        [width - 27., 9., 20., 20.],
        Action::Enabled,
        !state.settings.enabled,
    );

    let active = state.ui.tab;
    for (i, (tab, key)) in [
        (Tab::Devices, "devices"),
        (Tab::Volume, "volumes"),
        (Tab::Activation, "activation"),
        (Tab::Overlay, "overlay"),
        (Tab::Advanced, "advanced"),
        (Tab::Hotkeys, "hotkeys"),
    ]
    .into_iter()
    .enumerate()
    {
        let _ = key;
        let label = state.ui.tabs.labels[i].clone();
        let r = state.ui.tabs.rects[i];
        let [tx, ty, _, _] = r;
        panel(g, b, r, Rect(r).has(mouse), active == tab);
        small_icon(
            g,
            b,
            i,
            tx + 6.,
            ty + 6.,
            if active == tab { DIM } else { WHITE },
        );
        text(
            g,
            b,
            &label,
            tx + 18.,
            ty + 6.,
            if active == tab { DIM } else { WHITE },
        );
        widgets::record(&label, r, active != tab);
        if active != tab {
            state.ui.widgets.push(Widget {
                rect: Rect(r),
                action: Action::Tab(tab),
                slider: None,
            });
        }
    }
    let top = state.ui.tabs.content_top;
    let bottom = height - 6.;
    let w = 304f32.min(width - 28.);
    let x = (width - w) * 0.5 - 1.;

    let s = state.settings.clone();
    let v = state.snapshot.clone();
    let preview = state.preview;
    let default_device = tr("default");
    let mut activations = v.activations.to_vec();
    activations.sort_by(|a, b| (a.weight, &a.name).cmp(&(b.weight, &b.name)));
    let mut lines = v.lines.to_vec();
    lines.sort_by(|a, b| (a.weight, &a.name).cmp(&(b.weight, &b.name)));
    let activation_names: Vec<_> = activations
        .iter()
        .map(|a| activation_label(state, a))
        .collect();
    let line_names: Vec<_> = lines
        .iter()
        .map(|l| {
            let fallback = if l.name == "proximity" {
                tr("proximity")
            } else {
                l.name.clone()
            };
            server_label(state, &l.translation, &fallback)
        })
        .collect();
    let scroll = state.ui.scroll;
    let mut f = Form {
        g,
        b,
        m: &mut state.ui,
        mouse,
        x,
        w,
        y: top - scroll,
        top,
        bottom,
    };
    match active {
        Tab::Devices => {
            f.title("microphone");
            if f.shown() {
                f.label(&tr("threshold"));
                let mut r = f.rect();
                r[2] -= 24.;
                f.slider_at(&tr("threshold"), Slider::Threshold, s.threshold_db, "dB", r);
                let level = horizon_plasmo_engine::activation::db(v.level);
                let meter = ((level + 60.) / 60.).clamp(0., 1.);
                fill(
                    g,
                    f.b,
                    [r[0] + 2., r[1] + 16., (r[2] - 4.) * meter, 2.],
                    if level >= s.threshold_db {
                        0xff00a600
                    } else {
                        0xff006600
                    },
                );
                let icon = [r[0] + r[2] + 4., r[1], 20., 20.];
                panel(g, f.b, icon, Rect(icon).has(mouse), false);
                microphone(
                    g,
                    f.b,
                    icon[0] + 3.,
                    icon[1] + 2.,
                    1.,
                    if preview { GREEN } else { WHITE },
                    !preview,
                );
                widgets::record(&tr("test_microphone"), icon, true);
                f.m.widgets.push(Widget {
                    rect: Rect(icon),
                    action: Action::Preview,
                    slider: None,
                });
                f.reset(
                    Action::Value(Slider::Threshold, -30.),
                    (s.threshold_db + 30.).abs() > 0.001,
                );
            }
            f.y += 24.;
            f.combo(
                &tr("input"),
                if s.input_device.is_empty() {
                    default_device.as_str()
                } else {
                    &s.input_device
                },
                Choice::Input,
                0,
                !s.input_device.is_empty(),
            );
            f.slider(&tr("gain"), Slider::Gain, s.microphone_gain, 1., "%");
            for (key, flag) in [
                ("noise_suppression", Flag::Noise),
                ("stereo_capture", Flag::Stereo),
                ("disable_input", Flag::DisableInput),
            ] {
                f.toggle(&tr(key), flag, &s);
            }
            f.y += 3.;
            f.title("output");
            f.combo(
                &tr("output"),
                if s.output_device.is_empty() {
                    default_device.as_str()
                } else {
                    &s.output_device
                },
                Choice::Output,
                0,
                !s.output_device.is_empty(),
            );
            f.slider(&tr("volume"), Slider::Volume, s.volume, 1., "%");
            for (key, flag) in [
                ("sound_occlusion", Flag::Occlusion),
                ("directional_sources", Flag::Directional),
                ("hrtf", Flag::Hrtf),
            ] {
                f.toggle(&tr(key), flag, &s);
            }
        }
        Tab::Volume => {
            f.title("sources_volume");
            for (line, label) in lines.iter().zip(&line_names) {
                f.volume(
                    label,
                    Slider::Line(line.id),
                    s.line_volumes
                        .get(&line.id)
                        .copied()
                        .unwrap_or(line.default_volume),
                    line.default_volume,
                    s.line_muted.contains(&line.id),
                    None,
                    state.icons.get(&(false, line.id)).copied(),
                );
            }
            f.y += 4.;
            f.title("players_volume");
            if f.shown() {
                let r = [x + 2., f.y, w - 4., 20.];
                fill(g, f.b, r, 0xffb0b0b0);
                fill(g, f.b, [r[0] + 1., r[1] + 1., r[2] - 2., 18.], 0xff000000);
                text(
                    g,
                    f.b,
                    &format!("{}_", f.m.search),
                    r[0] + 4.,
                    r[1] + 6.,
                    WHITE,
                );
                widgets::record_as(&tr("search"), r, true, Kind::Field);
                f.m.widgets.push(Widget {
                    rect: Rect(r),
                    action: Action::Search,
                    slider: None,
                });
            }
            f.y += 24.;
            let query = f.m.search.to_lowercase();
            for (uuid, name) in &state.players {
                if name.to_lowercase().contains(&query) {
                    f.volume(
                        name,
                        Slider::Player(*uuid),
                        s.player_volumes.get(uuid).copied().unwrap_or(1.),
                        1.,
                        s.player_muted.contains(uuid),
                        Some(
                            state
                                .faces
                                .get(uuid)
                                .map(String::as_str)
                                .unwrap_or_else(|| horizon_client::tab_list::default_skin(*uuid)),
                        ),
                        None,
                    );
                }
            }
        }
        Tab::Activation => {
            for (a, label) in activations.iter().zip(&activation_names) {
                f.title_text(label);
                let setting = s.activations.get(&a.id);
                let mode = setting.map_or(
                    if a.name == "proximity" {
                        s.mode
                    } else {
                        Mode::Inherit
                    },
                    |v| v.mode,
                );
                f.combo(
                    &tr("activation_mode"),
                    &mode_label(mode),
                    Choice::Mode(a.id),
                    if a.name == "proximity" { 0 } else { 2 },
                    mode != if a.name == "proximity" {
                        Mode::PushToTalk
                    } else {
                        Mode::Inherit
                    },
                );
                let name = if a.name == "proximity" {
                    "talk".to_owned()
                } else {
                    format!("activation:{}", a.id)
                };
                let key = if a.name == "proximity" {
                    g.mod_key(ID, "talk", "key.keyboard.left.alt")
                } else {
                    setting.map_or(String::new(), |v| v.key.clone())
                };
                let value = if f.m.binding.as_deref() == Some(&name) {
                    tr("press_key")
                } else {
                    key_label(&key)
                };
                f.row(
                    &tr("talk_key"),
                    &value,
                    Action::Bind(name.clone()),
                    Action::ResetKey(name),
                    !key.is_empty() && key != "key.keyboard.left.alt",
                );
                if !a.distances.is_empty() {
                    let wanted = setting
                        .map_or(if a.name == "proximity" { s.distance } else { 0 }, |v| {
                            v.distance
                        });
                    let value = if wanted == 0 {
                        a.default_distance
                    } else {
                        wanted
                    };
                    let min = if a.dynamic_distance {
                        1
                    } else {
                        a.distances.iter().copied().min().unwrap_or(1)
                    };
                    let max = a.distances.iter().copied().max().unwrap_or(64);
                    f.slider(
                        &format!("{} \"{label}\"", tr("distance")),
                        Slider::Distance(a.id, min, max),
                        f32::from(value),
                        f32::from(a.default_distance),
                        "",
                    );
                }
                f.y += 3.;
            }
        }
        Tab::Overlay => {
            f.title("activation_icon");
            f.toggle(&tr("show_icons"), Flag::Hud, &s);
            f.row(
                &tr("hud_position"),
                &anchor_label(s.hud_position),
                Action::Position(true),
                Action::Anchor(true, Anchor::BottomCenter),
                s.hud_position != Anchor::BottomCenter,
            );
            f.y += 3.;
            f.title("entity_icons");
            f.combo(
                &tr("display_icons"),
                &tr(match s.icon_visibility {
                    IconVisibility::GuiOnly => "gui_only",
                    IconVisibility::Always => "always",
                    IconVisibility::Off => "hidden",
                }),
                Choice::Icons,
                0,
                s.icon_visibility != IconVisibility::GuiOnly,
            );
            f.toggle(&tr("static_icons"), Flag::StaticIcons, &s);
            f.y += 3.;
            f.title("overlay");
            f.toggle(&tr("speaker_overlay"), Flag::Overlay, &s);
            f.row(
                &tr("overlay_position"),
                &anchor_label(s.overlay_position),
                Action::Position(false),
                Action::Anchor(false, Anchor::TopLeft),
                s.overlay_position != Anchor::TopLeft,
            );
            f.combo(
                &tr("overlay_style"),
                &tr(match s.overlay_style {
                    OverlayStyle::NameSkin => "name_skin",
                    OverlayStyle::Name => "name_only",
                    OverlayStyle::Skin => "skin_only",
                }),
                Choice::Style,
                0,
                s.overlay_style != OverlayStyle::NameSkin,
            );
            f.y += 3.;
            f.title("overlay_sources");
            for (line, label) in lines.iter().zip(&line_names) {
                let mode = s.line_overlays.get(&line.id).copied().unwrap_or_default();
                if line.players.is_none() {
                    f.overlay_toggle(label, line.id, mode != OverlayMode::Off);
                } else {
                    f.combo(
                        label,
                        &tr(match mode {
                            OverlayMode::Off => "off",
                            OverlayMode::Talking => "when_talking",
                            OverlayMode::Always => "always",
                        }),
                        Choice::LineOverlay(line.id),
                        0,
                        mode != OverlayMode::Off,
                    );
                }
            }
        }
        Tab::Advanced => {
            f.title("visual_interface");
            f.toggle(&tr("visualize_distance"), Flag::Visualize, &s);
            f.toggle(&tr("visualize_join"), Flag::VisualizeJoin, &s);
            f.y += 3.;
            f.title("sound_engine");
            f.slider(
                &tr("directional_angle"),
                Slider::Angle,
                s.directional_angle,
                145.,
                "",
            );
            f.toggle(&tr("mono_sources"), Flag::Mono, &s);
            f.toggle(&tr("panning"), Flag::Panning, &s);
            f.combo(
                &tr("source_overlap"),
                match s.overlap {
                    SourceOverlap::MuteDirect => "Mute Direct",
                    SourceOverlap::MuteProximity => "Mute Proximity",
                    SourceOverlap::Off => "Off",
                },
                Choice::Overlap,
                1,
                s.overlap != SourceOverlap::MuteProximity,
            );
            f.toggle(&tr("adaptive_jitter"), Flag::Adaptive, &s);
            f.y += 3.;
            f.title("exponential_volume");
            f.toggle(&tr("volume_slider"), Flag::ExponentialSlider, &s);
            f.toggle(&tr("distance_gain"), Flag::ExponentialDistance, &s);
        }
        Tab::Hotkeys => {
            f.title("general");
            for (name, label, default) in [
                ("mute", "mute", "key.keyboard.m"),
                ("deafen", "disable_voice", ""),
                ("action", "action_key", "key.mouse.right"),
            ] {
                let key = g.mod_key(ID, name, default);
                let value = if f.m.binding.as_deref() == Some(name) {
                    tr("press_key")
                } else {
                    key_label(&key)
                };
                f.row(
                    &tr(label),
                    &value,
                    Action::Bind(name.into()),
                    Action::ResetKey(name.into()),
                    key != default,
                );
            }
            f.y += 3.;
            f.title("occlusion_hotkeys");
            let key = g.mod_key(ID, "occlusion", "");
            let value = if f.m.binding.as_deref() == Some("occlusion") {
                tr("press_key")
            } else {
                key_label(&key)
            };
            f.row(
                &tr("toggle_occlusion"),
                &value,
                Action::Bind("occlusion".into()),
                Action::ResetKey("occlusion".into()),
                !key.is_empty(),
            );
            for (a, label) in activations.iter().zip(&activation_names) {
                if a.distances.is_empty() {
                    continue;
                }
                f.y += 3.;
                f.title_text(&format!("{} \"{label}\"", tr("voice_distance")));
                for (suffix, key) in [
                    ("increase", "increase_distance"),
                    ("decrease", "decrease_distance"),
                ] {
                    let name = format!("{suffix}:{}", a.id);
                    let bound = g.mod_key(ID, &name, "");
                    let value = if f.m.binding.as_deref() == Some(&name) {
                        tr("press_key")
                    } else {
                        key_label(&bound)
                    };
                    f.row(
                        &tr(key),
                        &value,
                        Action::Bind(name.clone()),
                        Action::ResetKey(name),
                        !bound.is_empty(),
                    );
                }
            }
        }
    }
    let total = f.y + f.m.scroll - top;
    f.m.max_scroll = (total - (bottom - top)).max(0.);
    f.m.scroll = f.m.scroll.min(f.m.max_scroll);
    if let Some(choice) = f.m.dropdown {
        let anchor =
            f.m.widgets
                .iter()
                .find(|w| matches!(w.action,Action::Open(c)if c==choice))
                .map(|w| w.rect.0);
        if let Some([dx, dy, dw, _]) = anchor {
            let options: Vec<(String, Action)> = match choice {
                Choice::Input | Choice::Output => {
                    let input = choice == Choice::Input;
                    let devices = if input { &v.inputs } else { &v.outputs };
                    let mut values = vec![String::new()];
                    for d in devices {
                        if !values.contains(d) {
                            values.push(d.clone());
                        }
                    }
                    values
                        .into_iter()
                        .map(|d| {
                            (
                                if d.is_empty() {
                                    tr("default")
                                } else {
                                    d.clone()
                                },
                                if input {
                                    Action::Input(d)
                                } else {
                                    Action::Output(d)
                                },
                            )
                        })
                        .collect()
                }
                _ => {
                    let values: Vec<String> = match choice {
                        Choice::Mode(id) => [Mode::PushToTalk, Mode::Voice, Mode::Inherit]
                            .into_iter()
                            .take(
                                if v.activations
                                    .iter()
                                    .any(|a| a.id == id && a.name == "proximity")
                                {
                                    2
                                } else {
                                    3
                                },
                            )
                            .map(mode_label)
                            .collect(),
                        Choice::Icons => ["gui_only", "always", "hidden"].map(tr).to_vec(),
                        Choice::Style => ["name_skin", "name_only", "skin_only"].map(tr).to_vec(),
                        Choice::Overlap => ["Mute Direct", "Mute Proximity", "Off"]
                            .map(String::from)
                            .to_vec(),
                        Choice::LineOverlay(_) => {
                            ["off", "when_talking", "always"].map(tr).to_vec()
                        }
                        _ => Vec::new(),
                    };
                    values
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| (v, Action::Choose(choice, i)))
                        .collect()
                }
            };
            let visible = ((height - top - 24.) / 17.).max(1.) as usize;
            f.m.drop_scroll = f.m.drop_scroll.min(options.len().saturating_sub(visible));
            let count = options.len().min(visible);
            let dh = count as f32 * 17.;
            let py = if dy + 20. + dh < height {
                dy + 20.
            } else {
                (dy - dh).max(top)
            };
            fill(g, f.b, [dx, py, dw, dh + 1.], 0xffa0a0a0);
            for (i, (label, a)) in options
                .into_iter()
                .skip(f.m.drop_scroll)
                .take(visible)
                .enumerate()
            {
                let r = [dx + 1., py + i as f32 * 17. + 1., dw - 2., 16.];
                fill(
                    g,
                    f.b,
                    r,
                    if Rect(r).has(mouse) {
                        0xff303030
                    } else {
                        0xff000000
                    },
                );
                text(g, f.b, &trim(g, &label, dw - 8.), dx + 4., r[1] + 4., WHITE);
                widgets::record(&label, r, true);
                f.m.widgets.push(Widget {
                    rect: Rect(r),
                    action: a,
                    slider: None,
                });
            }
        }
    }
    if f.m.dropdown.is_none()
        && let Some(hint) = &f.m.tooltip
    {
        let mut lines = Vec::new();
        for paragraph in hint.split('\n') {
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let test = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if g.text_width(&test) > 176. && !line.is_empty() {
                    lines.push(horizon_client::text::Text::literal(std::mem::take(
                        &mut line,
                    )));
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                line.push_str(word);
            }
            lines.push(horizon_client::text::Text::literal(line));
        }
        g.gui_tooltip(f.b, &lines, mouse);
    }
}

fn icon_button(
    game: &dyn Host,
    batch: &mut HudBatch,
    ui: &mut Menu,
    mouse: [f32; 2],
    rect: [f32; 4],
    action: Action,
    selected: bool,
) {
    panel(game, batch, rect, Rect(rect).has(mouse), selected);
    let deafen = matches!(action, Action::Enabled);
    let color = if selected { RED } else { WHITE };
    if deafen {
        speaker(game, batch, rect[0] + 3., rect[1] + 2., 1., color, selected);
    } else {
        microphone(
            game,
            batch,
            rect[0] + 3.0,
            rect[1] + 2.0,
            1.0,
            color,
            selected,
        );
    }
    widgets::record(
        &tr(if deafen { "disable_voice" } else { "mute_key" }),
        rect,
        true,
    );
    ui.widgets.push(Widget {
        rect: Rect(rect),
        action,
        slider: None,
    });
}
// Merge adjacent cells of the same colour, snapping edges to physical pixels.
// The glyph remains pixel art at fractional GUI/world scales as well.
#[allow(clippy::too_many_arguments)]
fn pixel_icon(
    g: &dyn Host,
    b: &mut HudBatch,
    glyph: Glyph,
    mark: Mark,
    x: f32,
    y: f32,
    scale: f32,
    color: u32,
) {
    pixel_icon_grid(g, b, glyph, mark, x, y, scale, color, true);
}
fn pixel_icon_grid(
    g: &dyn Host,
    b: &mut HudBatch,
    glyph: Glyph,
    mark: Mark,
    x: f32,
    y: f32,
    scale: f32,
    color: u32,
    snap_pixels: bool,
) {
    let pixels = art::sprite(glyph, color, mark);
    let dpi = b.scale;
    let snap = |v: f32| {
        if snap_pixels {
            (v * dpi).round() / dpi
        } else {
            v
        }
    };
    for (row, cells) in pixels.iter().enumerate() {
        let mut col = 0;
        while col < 16 {
            let color = cells[col];
            let start = col;
            while col < 16 && cells[col] == color {
                col += 1;
            }
            if color != 0 {
                let x0 = snap(x + start as f32 * scale);
                let y0 = snap(y + row as f32 * scale);
                let x1 = snap(x + col as f32 * scale);
                let y1 = snap(y + (row + 1) as f32 * scale);
                if x1 > x0 && y1 > y0 {
                    fill(g, b, [x0, y0, x1 - x0, y1 - y0], color);
                }
            }
        }
    }
}
fn headphones(g: &dyn Host, b: &mut HudBatch, x: f32, y: f32, s: f32, c: u32, muted: bool) {
    pixel_icon(
        g,
        b,
        Glyph::Headphones,
        if muted { Mark::Muted } else { Mark::None },
        x,
        y,
        s,
        c,
    );
}
pub(super) fn microphone(
    g: &dyn Host,
    b: &mut HudBatch,
    x: f32,
    y: f32,
    s: f32,
    c: u32,
    muted: bool,
) {
    pixel_icon(
        g,
        b,
        Glyph::Microphone,
        if muted { Mark::Muted } else { Mark::None },
        x,
        y,
        s,
        c,
    );
}
#[derive(Clone, Copy)]
enum VoiceIcon {
    PttHeld,
    Microphone,
    ClientMute,
    ServerMute,
    InputUnavailable,
    Connection,
    Speaker,
    PlayerMute,
    PlayerServerMute,
    NoMod,
    Disabled,
}
fn speaker(g: &dyn Host, b: &mut HudBatch, x: f32, y: f32, s: f32, c: u32, muted: bool) {
    pixel_icon(
        g,
        b,
        Glyph::Speaker,
        if muted { Mark::Muted } else { Mark::None },
        x,
        y,
        s,
        c,
    );
}
fn voice_icon(g: &dyn Host, b: &mut HudBatch, icon: VoiceIcon, x: f32, y: f32, s: f32) {
    voice_icon_grid(g, b, icon, x, y, s, true);
}
fn voice_icon_grid(
    g: &dyn Host,
    b: &mut HudBatch,
    icon: VoiceIcon,
    x: f32,
    y: f32,
    s: f32,
    snap: bool,
) {
    let (glyph, mark) = match icon {
        VoiceIcon::PttHeld => (Glyph::Microphone, Mark::None),
        VoiceIcon::Microphone => (Glyph::Microphone, Mark::Talking(art::phase())),
        VoiceIcon::ClientMute => (Glyph::Microphone, Mark::Muted),
        VoiceIcon::ServerMute => (Glyph::Microphone, Mark::Locked),
        VoiceIcon::InputUnavailable => (Glyph::Microphone, Mark::Missing),
        VoiceIcon::Connection => (Glyph::Headphones, Mark::Connecting(art::phase())),
        VoiceIcon::Speaker => (Glyph::Microphone, Mark::Talking(art::phase())),
        VoiceIcon::PlayerMute => (Glyph::Microphone, Mark::Muted),
        VoiceIcon::PlayerServerMute => (Glyph::Microphone, Mark::Locked),
        VoiceIcon::NoMod => (Glyph::Headphones, Mark::Missing),
        VoiceIcon::Disabled => (Glyph::Headphones, Mark::Muted),
    };
    pixel_icon_grid(g, b, glyph, mark, x, y, s, 0xf1f3db, snap);
}
pub(super) fn project(g: &dyn Host, point: [f64; 3]) -> Option<[f32; 3]> {
    let camera = crate::runtime::camera(g)?;
    project_camera(g, &camera, point)
}
fn project_camera(
    g: &dyn Host,
    camera: &horizon_client::mod_api::NativeCamera,
    point: [f64; 3],
) -> Option<[f32; 3]> {
    let p = [
        (point[0] - camera.position[0]) as f32,
        (point[1] - camera.position[1]) as f32,
        (point[2] - camera.position[2]) as f32,
        1.,
    ];
    fn mul(m: [f32; 16], p: [f32; 4]) -> [f32; 4] {
        std::array::from_fn(|r| (0..4).map(|i| m[i * 4 + r] * p[i]).sum())
    }
    let p = mul(camera.projection, mul(camera.view, p));
    if p[3] <= 0.001 {
        return None;
    }
    let x = p[0] / p[3];
    let y = p[1] / p[3];
    if !x.is_finite() || !y.is_finite() || x.abs() > 1.2 || y.abs() > 1.2 {
        return None;
    }
    let (w, h) = g.gui_size();
    Some([(x + 1.) * 0.5 * w, (1. - y) * 0.5 * h, 1. / p[3]])
}
// HUD quads do not share the world's depth buffer. Clip the camera-to-icon
// segment against the host's visual block shapes before submitting them.
// This also handles slabs/stairs and leaves transparent glass unobstructed.
fn visible_project(
    g: &dyn Host,
    camera: &horizon_client::mod_api::NativeCamera,
    point: [f64; 3],
) -> Option<[f32; 3]> {
    let projected = project_camera(g, camera, point)?;
    g.client_visual_clear(camera.position, point)
        .then_some(projected)
}
fn face(g: &dyn Host, state: &State, b: &mut HudBatch, uuid: u128, x: f32, y: f32, size: i32) {
    let default = horizon_client::tab_list::default_skin(uuid);
    let skin = state
        .faces
        .get(&uuid)
        .map(String::as_str)
        .unwrap_or(default);
    paint_face(g, b, skin, x, y, size);
}
fn paint_face(g: &dyn Host, b: &mut HudBatch, skin: &str, x: f32, y: f32, size: i32) {
    let mut draws = Vec::new();
    horizon_client::tab_list::face(
        &mut draws,
        skin,
        x as i32,
        y as i32,
        size,
        true,
        false,
        u32::MAX,
    );
    g.gui_paint(b, &draws);
}
// Draw once after every screen, so pause/options cannot cover the microphone.
pub(super) fn activation_hud(g: &dyn Host, state: &State, b: &mut HudBatch) {
    // No voice handshake has arrived. Ordinary servers must not display a
    // permanent connection animation, including when PTT or mute is held.
    if state.status == Status::Waiting {
        return;
    }
    let (w, h) = g.gui_size();
    let s = &state.settings;
    let v = &state.snapshot;
    if s.show_hud
        && (v.transmitting
            || state.ptt_held()
            || s.muted
            || s.deafened
            || !s.enabled
            || state.status != Status::Connected
            || v.permissions.get("pv.activation.proximity") == Some(&false))
    {
        let targets: Vec<_> = if v.transmitting {
            v.transmitting_activations.clone()
        } else {
            Vec::new()
        };
        let count = targets.len().max(1);
        let size = 16. * s.hud_scale;
        let [x, mut y] = s.hud_position.point(w, h, size * count as f32, size);
        if s.hud_position == Anchor::BottomCenter {
            y = h - 60.;
        }
        let icon = if !s.enabled || s.deafened {
            VoiceIcon::Disabled
        } else if state.status == Status::Connecting {
            VoiceIcon::Connection
        } else if matches!(state.status, Status::TimedOut | Status::Error) {
            VoiceIcon::NoMod
        } else if state.status == Status::ListenOnly {
            VoiceIcon::InputUnavailable
        } else if v.permissions.get("pv.activation.proximity") == Some(&false) {
            VoiceIcon::ServerMute
        } else if s.muted {
            VoiceIcon::ClientMute
        } else if v.transmitting {
            VoiceIcon::Microphone
        } else {
            VoiceIcon::PttHeld
        };
        for i in 0..count {
            let at = x + i as f32 * size;
            if v.transmitting
                && let Some(rect) = targets.get(i).and_then(|id| state.icons.get(&(true, *id)))
            {
                g.sprite(b, "horizon/modimage", *rect, [at, y, size, size]);
            } else {
                voice_icon(g, b, icon, at, y, s.hud_scale);
            }
        }
    }
}

pub(super) fn hud(g: &dyn Host, state: &State, b: &mut HudBatch) {
    let (w, h) = g.gui_size();
    let s = &state.settings;
    let v = &state.snapshot;
    if let Some(hint) = &state.volume_hint
        && hint.at.elapsed().as_secs_f32() < 1.5
    {
        let volume = s.player_volumes.get(&hint.uuid).copied().unwrap_or(1.);
        let label = format!("{}: {:.0}%", hint.name, volume * 100.);
        let width = g.text_width(&label).max(108.) + 28.;
        let x = (w - width) * 0.5;
        let y = h * 0.5 + 24.;
        fill(g, b, [x - 4., y - 4., width + 8., 26.], 0xbd12191c);
        speaker(g, b, x, y, 0.75, WHITE, s.player_muted.contains(&hint.uuid));
        text(g, b, &label, x + 18., y + 2., WHITE);
        fill(g, b, [x + 18., y + 16., width - 18., 2.], 0xff4b5659);
        fill(
            g,
            b,
            [x + 18., y + 16., (width - 18.) * volume / 2., 2.],
            0xff79e38d,
        );
    }
    world_icons(g, state, b);
    if let Some(viz) = &state.visualization {
        let age = viz.at.elapsed().as_secs_f32();
        if age < 3. {
            let alpha = ((1. - age / 3.) * 190.) as u32;
            let color = (viz.color & 0xffffff) | (alpha << 24);
            let mut previous: Option<[f32; 3]> = None;
            for i in 0..=96 {
                let angle = i as f64 * std::f64::consts::TAU / 96.;
                let r = f64::from(viz.radius);
                let p = project(
                    g,
                    [
                        viz.center[0] + angle.cos() * r,
                        viz.center[1] + 0.1,
                        viz.center[2] + angle.sin() * r,
                    ],
                );
                if let (Some(a), Some(c)) = (previous, p) {
                    let dx = c[0] - a[0];
                    let dy = c[1] - a[1];
                    let len = (dx * dx + dy * dy).sqrt();
                    if len > 0.01 && len < 100. {
                        let nx = -dy / len;
                        let ny = dx / len;
                        g.quad(
                            b,
                            [
                                [a[0] + nx, a[1] + ny],
                                [c[0] + nx, c[1] + ny],
                                [c[0] - nx, c[1] - ny],
                                [a[0] - nx, a[1] - ny],
                            ],
                            color,
                        );
                    }
                }
                previous = p;
            }
        }
    }
    if !s.show_speakers || s.deafened || !s.enabled {
        return;
    }
    let mut rows = Vec::new();
    for line in v.lines.iter() {
        let mode = s
            .line_overlays
            .get(&line.id)
            .copied()
            .unwrap_or(OverlayMode::Off);
        if mode == OverlayMode::Off || s.line_muted.contains(&line.id) {
            continue;
        }
        if mode == OverlayMode::Always
            && let Some(players) = &line.players
        {
            for p in players {
                let talking = v
                    .speakers
                    .iter()
                    .any(|x| x.line == line.id && x.player == p.id && x.speaking && x.audible);
                rows.push((line.id, p.id, p.name.clone(), talking));
            }
        } else {
            for speaker in v
                .speakers
                .iter()
                .filter(|x| x.line == line.id && x.speaking && x.audible)
            {
                if !rows.iter().any(|r| r.0 == line.id && r.1 == speaker.player) {
                    rows.push((line.id, speaker.player, speaker.name.clone(), true));
                }
            }
        }
    }
    rows.retain(|r| {
        !s.player_muted.contains(&r.1) && s.player_volumes.get(&r.1).copied().unwrap_or(1.) > 0.
    });
    let names = s.overlay_style != OverlayStyle::Skin;
    let skins = s.overlay_style != OverlayStyle::Name;
    let width = rows
        .iter()
        .map(|r| if names { g.text_width(&r.2) + 4. } else { 0. })
        .fold(0f32, f32::max)
        + if skins { 20. } else { 0. }
        + 12.;
    let height = rows.len().min(12) as f32 * 20.;
    let [x, mut y] = s.overlay_position.point(w, h, width, height);
    for (_, uuid, name, talking) in rows.into_iter().take(12) {
        if skins {
            face(g, state, b, uuid, x, y, 16);
        }
        if names {
            text(g, b, &name, x + if skins { 20. } else { 0. }, y + 4., WHITE);
        }
        if talking {
            let ix = if names {
                x + if skins { 20. } else { 0. } + g.text_width(&name) + 4.
            } else {
                x + 20.
            };
            voice_icon(g, b, VoiceIcon::Speaker, ix, y + 2., 0.75);
        }
        y += 20.;
    }
}

pub(super) fn world_icons(g: &dyn Host, state: &State, b: &mut HudBatch) {
    let s = &state.settings;
    let v = &state.snapshot;
    if s.icon_visibility == IconVisibility::Off
        || (s.icon_visibility == IconVisibility::GuiOnly
            && g.client_state().is_some_and(|v| v.gui_hidden))
    {
        return;
    }
    let Some(camera) = crate::runtime::camera(g) else {
        return;
    };
    // A 0.32-block billboard: the same world size at every distance, with
    // ordinary perspective/FOV scaling and no near/far size clamps. Its cells
    // stay fractional in world rendering; snapping each one distorts small art.
    let (_, height) = g.gui_size();
    let unit_scale = crate::runtime::HEAD_ICON_SIZE / 16. * height * 0.5 * camera.projection[5];
    for (uuid, anchor) in &state.name_icons {
        let info = v.players.iter().find(|p| p.id == *uuid);
        let hide = |name: &str| v.hidden_icons.iter().any(|s| s == name);
        let icon = match info {
            None => {
                if hide("HIDE_NOT_INSTALLED") {
                    None
                } else {
                    Some(VoiceIcon::NoMod)
                }
            }
            Some(p) if p.voice_disabled => {
                if hide("HIDE_VOICE_CHAT_DISABLED") {
                    None
                } else {
                    Some(VoiceIcon::Disabled)
                }
            }
            Some(p) if p.server_muted => {
                if hide("HIDE_SERVER_MUTED") {
                    None
                } else {
                    Some(VoiceIcon::PlayerServerMute)
                }
            }
            Some(p) if p.microphone_muted || s.player_muted.contains(uuid) => {
                if hide("HIDE_CLIENT_MUTED") {
                    None
                } else {
                    Some(VoiceIcon::PlayerMute)
                }
            }
            _ => v
                .speakers
                .iter()
                .any(|p| p.player == *uuid && p.speaking && p.icon_visible)
                .then_some(VoiceIcon::Speaker),
        };
        if hide("HIDE_SOURCE_ICON") && matches!(icon, Some(VoiceIcon::Speaker)) {
            continue;
        }
        if let Some(icon) = icon
            && let Some(p) = visible_project(g, &camera, anchor.position)
        {
            let scale = p[2] * unit_scale;
            voice_icon_grid(
                g,
                b,
                icon,
                p[0] - 8. * scale,
                p[1] - 8. * scale,
                scale,
                false,
            );
        }
    }
    if s.static_icons {
        for source in v
            .speakers
            .iter()
            .filter(|p| p.static_source && p.speaking && p.icon_visible)
        {
            if let Some(p) = source.position.and_then(|p| visible_project(g, &camera, p)) {
                let scale = p[2] * unit_scale;
                if let Some(rect) = state.icons.get(&(false, source.line)) {
                    g.sprite(
                        b,
                        "horizon/modimage",
                        *rect,
                        [
                            p[0] - 8. * scale,
                            p[1] - 8. * scale,
                            16. * scale,
                            16. * scale,
                        ],
                    );
                } else {
                    voice_icon_grid(
                        g,
                        b,
                        VoiceIcon::Speaker,
                        p[0] - 8. * scale,
                        p[1] - 8. * scale,
                        scale,
                        false,
                    );
                }
            }
        }
    }
}
