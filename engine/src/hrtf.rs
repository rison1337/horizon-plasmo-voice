//! OpenAL Soft loopback renderer. HRTF is rendered into the ordinary CPAL
//! output, so this opens no playback/capture device and keeps device selection.
//! Uses the published OpenAL 1.1, SOFT_loopback, SOFT_HRTF and thread-local APIs.
use crate::Result;
use libloading::Library;
use std::{
    collections::{HashMap, HashSet},
    ffi::{c_char, c_void},
    ptr,
    sync::OnceLock,
};

type Ptr = *mut c_void;
type Proc = unsafe extern "C" fn(Ptr, *const c_char) -> Ptr;
unsafe fn function<T: Copy>(lib: &Library, proc: Proc, name: &[u8]) -> Result<T> {
    // SAFETY: callers supply a NUL-terminated name and its published C ABI.
    if let Ok(symbol) = unsafe { lib.get::<T>(name) } {
        return Ok(*symbol);
    }
    let address = unsafe { proc(ptr::null_mut(), name.as_ptr().cast()) };
    if address.is_null() {
        return Err(format!(
            "OpenAL extension missing: {}",
            String::from_utf8_lossy(name)
        ));
    }
    if std::mem::size_of::<T>() != std::mem::size_of::<Ptr>() {
        return Err("invalid OpenAL symbol type".into());
    }
    Ok(unsafe { std::mem::transmute_copy(&address) })
}
macro_rules! api {
    ($($field:ident:$ty:ty=$symbol:literal),+ $(,)?)=>{
        struct Api { $($field:$ty,)+ _library:&'static Library }
        impl Api {fn load()->Result<Self>{
            #[cfg(target_os="linux")] const NAMES:&[&str]=&["libopenal.so.1","libopenal.so"];
            #[cfg(target_os="windows")] const NAMES:&[&str]=&["soft_oal.dll","OpenAL32.dll","libopenal-1.dll"];
            #[cfg(target_os="macos")] const NAMES:&[&str]=&["libopenal.dylib","/opt/homebrew/lib/libopenal.dylib"];
            // SAFETY: signatures are the documented public API; the loaded
            // library outlives every context, source and buffer using it.
            unsafe {
                // OpenAL registers thread-exit destructors. Their function
                // pointers must stay resident after a context is closed.
                static LIBRARY:OnceLock<std::result::Result<Library,String>>=OnceLock::new();
                let library=LIBRARY.get_or_init(||crate::libraries::load(NAMES)).as_ref().map_err(Clone::clone)?;
                let proc:Proc=*library.get(b"alcGetProcAddress\0").map_err(|e|e.to_string())?;
                Ok(Self{$($field:function(library,proc,concat!($symbol,"\0").as_bytes())?,)+_library:library})
            }
        }}
    }
}
api! {
    open:unsafe extern "C" fn(*const c_char)->Ptr="alcLoopbackOpenDeviceSOFT",
    create:unsafe extern "C" fn(Ptr,*const i32)->Ptr="alcCreateContext",
    destroy:unsafe extern "C" fn(Ptr)="alcDestroyContext",
    close:unsafe extern "C" fn(Ptr)->u8="alcCloseDevice",
    current:unsafe extern "C" fn(Ptr)->u8="alcSetThreadContext",
    integer:unsafe extern "C" fn(Ptr,i32,i32,*mut i32)="alcGetIntegerv",
    render:unsafe extern "C" fn(Ptr,Ptr,i32)="alcRenderSamplesSOFT",
    gen_sources:unsafe extern "C" fn(i32,*mut u32)="alGenSources",
    delete_sources:unsafe extern "C" fn(i32,*const u32)="alDeleteSources",
    gen_buffers:unsafe extern "C" fn(i32,*mut u32)="alGenBuffers",
    delete_buffers:unsafe extern "C" fn(i32,*const u32)="alDeleteBuffers",
    data:unsafe extern "C" fn(u32,i32,*const c_void,i32,i32)="alBufferData",
    source_i:unsafe extern "C" fn(u32,i32,i32)="alSourcei",
    source_f:unsafe extern "C" fn(u32,i32,f32)="alSourcef",
    source_3f:unsafe extern "C" fn(u32,i32,f32,f32,f32)="alSource3f",
    get_source_i:unsafe extern "C" fn(u32,i32,*mut i32)="alGetSourcei",
    queue:unsafe extern "C" fn(u32,i32,*const u32)="alSourceQueueBuffers",
    unqueue:unsafe extern "C" fn(u32,i32,*mut u32)="alSourceUnqueueBuffers",
    play:unsafe extern "C" fn(u32)="alSourcePlay",
    stop:unsafe extern "C" fn(u32)="alSourceStop",
    distance:unsafe extern "C" fn(i32)="alDistanceModel",
    error:unsafe extern "C" fn()->i32="alGetError",
}
struct Source {
    id: u32,
    buffers: Vec<u32>,
    free: Vec<u32>,
}
pub struct Hrtf {
    api: Api,
    device: Ptr,
    context: Ptr,
    sources: HashMap<u128, Source>,
    buffers: u8,
}
impl Hrtf {
    pub fn new(buffers: u8) -> Result<Self> {
        let api = Api::load()?;
        // SAFETY: loopback is a software renderer; all lists are correctly
        // terminated, outputs are live, and failed allocations are cleaned up.
        unsafe {
            let device = (api.open)(ptr::null());
            if device.is_null() {
                return Err("OpenAL loopback unavailable".into());
            }
            let attributes = [0x1007, 48000, 0x1990, 0x1501, 0x1991, 0x1406, 0x1992, 1, 0];
            let context = (api.create)(device, attributes.as_ptr());
            if context.is_null() {
                (api.close)(device);
                return Err("OpenAL HRTF context unavailable".into());
            }
            if (api.current)(context) == 0 {
                (api.destroy)(context);
                (api.close)(device);
                return Err("cannot select OpenAL thread context".into());
            }
            let mut enabled = 0;
            (api.integer)(device, 0x1992, 1, &mut enabled);
            if enabled == 0 {
                (api.current)(ptr::null_mut());
                (api.destroy)(context);
                (api.close)(device);
                return Err("OpenAL could not enable HRTF".into());
            }
            (api.distance)(0);
            Ok(Self {
                api,
                device,
                context,
                sources: HashMap::new(),
                buffers: buffers.clamp(2, 12),
            })
        }
    }
    pub fn push(&mut self, id: u128, pcm: &[i16], position: [f32; 3], gain: f32) -> Result<()> {
        if pcm.len() != 960 || position.iter().any(|v| !v.is_finite()) || !gain.is_finite() {
            return Err("invalid HRTF input".into());
        }
        // OpenAL caps source gain at AL_MAX_GAIN (1 by default). Bake the
        // per-player/channel gain into float PCM instead, preserving boosts
        // above 100% and headroom without saturating a 16-bit buffer.
        let amplified: [f32; 960] = std::array::from_fn(|i| {
            f32::from(pcm[i]) / 32768. * gain.clamp(0., 64. * std::f32::consts::SQRT_2)
        });
        // SAFETY: this selects only our worker's context. Source IDs and
        // buffer IDs belong to that context; all pointer lengths are bounded.
        unsafe {
            (self.api.current)(self.context);
            if !self.sources.contains_key(&id) {
                if self.sources.len() >= 64 {
                    return Err("HRTF source limit".into());
                }
                let mut source = 0;
                (self.api.gen_sources)(1, &mut source);
                let mut buffers = vec![0; usize::from(self.buffers)];
                (self.api.gen_buffers)(buffers.len() as i32, buffers.as_mut_ptr());
                (self.api.source_i)(source, 0x202, 1); // AL_SOURCE_RELATIVE
                let mut free = buffers.clone();
                let prime = free.pop().unwrap();
                let silence = [0f32; 960];
                (self.api.data)(prime, 0x10010, silence.as_ptr().cast(), 3840, 48000);
                (self.api.queue)(source, 1, &prime);
                self.sources.insert(
                    id,
                    Source {
                        id: source,
                        buffers,
                        free,
                    },
                );
            }
            let source = self.sources.get_mut(&id).unwrap();
            let mut processed = 0;
            (self.api.get_source_i)(source.id, 0x1016, &mut processed);
            for _ in 0..processed.clamp(0, 12) {
                let mut buffer = 0;
                (self.api.unqueue)(source.id, 1, &mut buffer);
                source.free.push(buffer);
            }
            (self.api.source_3f)(source.id, 0x1004, position[0], position[1], position[2]);
            (self.api.source_f)(source.id, 0x100a, 1.);
            if let Some(buffer) = source.free.pop() {
                (self.api.data)(buffer, 0x10010, amplified.as_ptr().cast(), 3840, 48000);
                (self.api.queue)(source.id, 1, &buffer);
            }
            let mut state = 0;
            (self.api.get_source_i)(source.id, 0x1010, &mut state);
            if state != 0x1012 {
                (self.api.play)(source.id);
            }
            let error = (self.api.error)();
            if error != 0 {
                return Err(format!("OpenAL HRTF error {error:#x}"));
            }
        }
        Ok(())
    }
    pub fn render(&mut self) -> Vec<f32> {
        let mut stereo = vec![0.; 1920];
        // SAFETY: 960 interleaved stereo frames in the configured float format.
        unsafe {
            (self.api.current)(self.context);
            (self.api.render)(self.device, stereo.as_mut_ptr().cast(), 960);
        }
        stereo
    }
    pub fn retain(&mut self, ids: &HashSet<u128>) {
        let removed: Vec<_> = self
            .sources
            .keys()
            .copied()
            .filter(|id| !ids.contains(id))
            .collect();
        for id in removed {
            if let Some(source) = self.sources.remove(&id) {
                self.delete(source);
            }
        }
    }
    fn delete(&self, source: Source) {
        // SAFETY: deleting the owning source releases queued buffers before
        // those buffers are destroyed, and every ID is owned exactly once.
        unsafe {
            (self.api.current)(self.context);
            (self.api.stop)(source.id);
            (self.api.delete_sources)(1, &source.id);
            (self.api.delete_buffers)(source.buffers.len() as i32, source.buffers.as_ptr());
        }
    }
}
impl Drop for Hrtf {
    fn drop(&mut self) {
        let sources: Vec<_> = self.sources.drain().map(|(_, s)| s).collect();
        for source in sources {
            self.delete(source);
        }
        // SAFETY: all children are gone; clear only our thread-local context,
        // then destroy the context and its loopback device before the library.
        unsafe {
            (self.api.current)(ptr::null_mut());
            (self.api.destroy)(self.context);
            (self.api.close)(self.device);
        }
    }
}
