//! Typed options shared by the native menu and the audio worker.
use crate::session::Settings;
macro_rules! flags {
    ($($variant:ident => $field:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Flag { $($variant),+ }
        impl Flag {
            pub fn get(self, settings: &Settings) -> bool {
                match self { $(Self::$variant => settings.$field),+ }
            }
            pub fn set(self, settings: &mut Settings, value: bool) {
                match self { $(Self::$variant => settings.$field = value),+ }
            }
            pub fn default_value(self) -> bool { self.get(&Settings::default()) }
        }
    };
}
flags! {
    Noise => noise_suppression,
    Stereo => stereo_capture,
    DisableInput => disable_input_device,
    Occlusion => sound_occlusion,
    Directional => directional_sources,
    Hrtf => hrtf,
    Hud => show_hud,
    StaticIcons => static_icons,
    Overlay => show_speakers,
    Visualize => visualize_distance,
    VisualizeJoin => visualize_on_join,
    Mono => mono_sources,
    Panning => panning,
    Adaptive => adaptive_jitter_buffer,
    ExponentialSlider => exponential_volume_slider,
    ExponentialDistance => exponential_distance_gain,
    Camera => camera_sound_listener,
    Compressor => compressor_limiter,
}
