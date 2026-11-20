//! Owning binding to the user's RNNoise through its published stable C API.
use crate::Result;
use libloading::Library;
use std::ffi::c_void;

pub struct Denoiser {
    state: *mut c_void,
    process: unsafe extern "C" fn(*mut c_void, *mut f32, *const f32) -> f32,
    destroy: unsafe extern "C" fn(*mut c_void),
    _library: Library,
}
impl Denoiser {
    pub fn load() -> Result<Self> {
        #[cfg(target_os = "linux")]
        const NAMES: &[&str] = &["librnnoise.so.0", "librnnoise.so"];
        #[cfg(target_os = "macos")]
        const NAMES: &[&str] = &["librnnoise.dylib", "/opt/homebrew/lib/librnnoise.dylib"];
        #[cfg(target_os = "windows")]
        const NAMES: &[&str] = &["rnnoise.dll", "librnnoise-0.dll"];
        // SAFETY: RNNoise's public C signatures; the library outlives the
        // state, which is accessed only by the session worker that owns it.
        unsafe {
            let library = crate::libraries::load(NAMES)?;
            let frame_size = library
                .get::<unsafe extern "C" fn() -> i32>(b"rnnoise_get_frame_size\0")
                .map_err(|e| e.to_string())?;
            if frame_size() != 480 {
                return Err("unsupported RNNoise frame size".into());
            }
            let create = library
                .get::<unsafe extern "C" fn(*mut c_void) -> *mut c_void>(b"rnnoise_create\0")
                .map_err(|e| e.to_string())?;
            let process = *library
                .get(b"rnnoise_process_frame\0")
                .map_err(|e| e.to_string())?;
            let destroy = *library
                .get(b"rnnoise_destroy\0")
                .map_err(|e| e.to_string())?;
            let state = create(std::ptr::null_mut());
            if state.is_null() {
                return Err("RNNoise could not allocate its model".into());
            }
            Ok(Self {
                state,
                process,
                destroy,
                _library: library,
            })
        }
    }
    /// RNNoise consumes 10 ms, 48 kHz mono frames in signed-16 amplitude units.
    pub fn process(&mut self, pcm: &mut [i16]) -> Result<()> {
        if !pcm.len().is_multiple_of(480) {
            return Err("invalid RNNoise input frame".into());
        }
        for frame in pcm.as_chunks_mut::<480>().0 {
            let input: [f32; 480] = std::array::from_fn(|i| f32::from(frame[i]));
            let mut output = [0.; 480];
            // SAFETY: both arrays contain the required 480 samples and the
            // owned state is uniquely borrowed for this call.
            unsafe {
                (self.process)(self.state, output.as_mut_ptr(), input.as_ptr());
            }
            for (sample, value) in frame.iter_mut().zip(output) {
                *sample = value.clamp(-32768., 32767.) as i16;
            }
        }
        Ok(())
    }
}
impl Drop for Denoiser {
    fn drop(&mut self) {
        // SAFETY: the state was created by this live library and is freed once.
        unsafe {
            (self.destroy)(self.state);
        }
    }
}
