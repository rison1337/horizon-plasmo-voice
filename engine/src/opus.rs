//! A small owning binding to the user's libopus; no codec runs on the game thread.
use crate::Result;
use libloading::Library;
use std::{
    ffi::{CStr, c_char, c_int, c_void},
    sync::Arc,
};

type Create = unsafe extern "C" fn(c_int, c_int, *mut c_int) -> *mut c_void;
type EncoderCreate = unsafe extern "C" fn(c_int, c_int, c_int, *mut c_int) -> *mut c_void;
type Encode = unsafe extern "C" fn(*mut c_void, *const i16, c_int, *mut u8, c_int) -> c_int;
type Decode = unsafe extern "C" fn(*mut c_void, *const u8, c_int, *mut i16, c_int, c_int) -> c_int;
type Destroy = unsafe extern "C" fn(*mut c_void);
type EncoderControl = unsafe extern "C" fn(*mut c_void, c_int, ...) -> c_int;

pub struct Opus {
    _library: Library,
    encoder_create: EncoderCreate,
    decoder_create: Create,
    encode: Encode,
    decode: Decode,
    encoder_destroy: Destroy,
    encoder_control: EncoderControl,
    decoder_destroy: Destroy,
    strerror: unsafe extern "C" fn(c_int) -> *const c_char,
}

impl Opus {
    pub fn load() -> Result<Arc<Self>> {
        #[cfg(target_os = "linux")]
        const NAMES: &[&str] = &["libopus.so.0", "libopus.so"];
        #[cfg(target_os = "macos")]
        const NAMES: &[&str] = &[
            "libopus.0.dylib",
            "libopus.dylib",
            "/opt/homebrew/lib/libopus.0.dylib",
            "/usr/local/lib/libopus.0.dylib",
        ];
        #[cfg(target_os = "windows")]
        const NAMES: &[&str] = &["opus.dll", "libopus-0.dll", "libopus.dll"];
        // SAFETY: symbols have libopus's documented stable C ABI; the Library
        // is retained until all codecs, and then their Arc<Opus>, are dropped.
        unsafe {
            let lib = crate::libraries::load(NAMES)?;
            Ok(Arc::new(Self {
                encoder_create: *lib
                    .get(b"opus_encoder_create\0")
                    .map_err(|e| e.to_string())?,
                decoder_create: *lib
                    .get(b"opus_decoder_create\0")
                    .map_err(|e| e.to_string())?,
                encode: *lib.get(b"opus_encode\0").map_err(|e| e.to_string())?,
                decode: *lib.get(b"opus_decode\0").map_err(|e| e.to_string())?,
                encoder_destroy: *lib
                    .get(b"opus_encoder_destroy\0")
                    .map_err(|e| e.to_string())?,
                encoder_control: *lib.get(b"opus_encoder_ctl\0").map_err(|e| e.to_string())?,
                decoder_destroy: *lib
                    .get(b"opus_decoder_destroy\0")
                    .map_err(|e| e.to_string())?,
                strerror: *lib.get(b"opus_strerror\0").map_err(|e| e.to_string())?,
                _library: lib,
            }))
        }
    }
    fn error(&self, code: i32) -> String {
        // SAFETY: opus_strerror returns a static NUL-terminated string.
        unsafe {
            CStr::from_ptr((self.strerror)(code))
                .to_string_lossy()
                .into_owned()
        }
    }
    pub fn encoder(self: &Arc<Self>, rate: u32) -> Result<Encoder> {
        self.configured_encoder(rate, None)
    }
    pub fn configured_encoder(
        self: &Arc<Self>,
        rate: u32,
        config: Option<&crate::protocol::Codec>,
    ) -> Result<Encoder> {
        let mut application = 2048;
        let mut bitrate = None;
        if let Some(config) = config {
            if !config.name.eq_ignore_ascii_case("opus") {
                return Err("unsupported capture codec".into());
            }
            application = match config
                .params
                .get("mode")
                .map(String::as_str)
                .unwrap_or("VOIP")
            {
                "VOIP" => 2048,
                "AUDIO" => 2049,
                "RESTRICTED_LOWDELAY" => 2051,
                _ => return Err("unsupported Opus application".into()),
            };
            if let Some(value) = config.params.get("bitrate") {
                let value = value.parse::<i32>().map_err(|_| "invalid Opus bitrate")?;
                if !matches!(value, -1000 | -1) && !(500..=512000).contains(&value) {
                    return Err("invalid Opus bitrate".into());
                }
                bitrate = Some(value);
            }
        }
        let mut error = 0;
        // SAFETY: valid integer arguments and a live output slot; result is owned.
        let ptr = unsafe { (self.encoder_create)(rate as i32, 1, application, &mut error) };
        if error != 0 || ptr.is_null() {
            return Err(self.error(error));
        }
        let encoder = Encoder {
            opus: self.clone(),
            ptr,
            samples: rate as usize / 50,
        };
        if let Some(bitrate) = bitrate {
            // SAFETY: OPUS_SET_BITRATE_REQUEST (4002) takes exactly one int.
            let result = unsafe { (self.encoder_control)(ptr, 4002, bitrate) };
            if result != 0 {
                return Err(self.error(result));
            }
        }
        Ok(encoder)
    }
    pub fn decoder(self: &Arc<Self>, stereo: bool) -> Result<Decoder> {
        let mut error = 0;
        let channels = if stereo { 2 } else { 1 };
        // SAFETY: supported sample rate and channel count, owned result.
        let ptr = unsafe { (self.decoder_create)(48000, channels, &mut error) };
        if error != 0 || ptr.is_null() {
            return Err(self.error(error));
        }
        Ok(Decoder {
            opus: self.clone(),
            ptr,
            channels: channels as usize,
        })
    }
}

pub struct Encoder {
    opus: Arc<Opus>,
    ptr: *mut c_void,
    samples: usize,
}
impl Encoder {
    pub fn encode(&mut self, pcm: &[i16], mtu: usize) -> Result<Vec<u8>> {
        if pcm.len() != self.samples || !(64..=4096).contains(&mtu) {
            return Err("invalid Opus frame size".into());
        }
        let mut out = vec![0; mtu];
        // SAFETY: the encoder is uniquely borrowed; both buffers have the advertised size.
        let n = unsafe {
            (self.opus.encode)(
                self.ptr,
                pcm.as_ptr(),
                pcm.len() as i32,
                out.as_mut_ptr(),
                mtu as i32,
            )
        };
        if n < 0 {
            return Err(self.opus.error(n));
        }
        out.truncate(n as usize);
        Ok(out)
    }
}
impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe {
            (self.opus.encoder_destroy)(self.ptr);
        }
    }
}

pub struct Decoder {
    opus: Arc<Opus>,
    ptr: *mut c_void,
    channels: usize,
}
impl Decoder {
    pub fn decode(&mut self, packet: Option<&[u8]>) -> Result<Vec<i16>> {
        let packet = packet.unwrap_or(&[]);
        if packet.len() > 4096 {
            return Err("Opus packet too large".into());
        }
        let capacity = if packet.is_empty() { 960 } else { 5760 };
        let mut out = vec![0; capacity * self.channels];
        let ptr = if packet.is_empty() {
            std::ptr::null()
        } else {
            packet.as_ptr()
        };
        // SAFETY: the decoder is unique, frame capacity is per channel, input is bounded.
        let n = unsafe {
            (self.opus.decode)(
                self.ptr,
                ptr,
                packet.len() as i32,
                out.as_mut_ptr(),
                capacity as i32,
                0,
            )
        };
        if n < 0 {
            return Err(self.opus.error(n));
        }
        out.truncate(n as usize * self.channels);
        Ok(out)
    }
    pub fn channels(&self) -> usize {
        self.channels
    }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe {
            (self.opus.decoder_destroy)(self.ptr);
        }
    }
}
