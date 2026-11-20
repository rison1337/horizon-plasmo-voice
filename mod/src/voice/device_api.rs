//! OpenAL 1.1 capture/playback and ALC_EXT_thread_local_context public ABIs.
//! The library stays resident for the lifetime of its driver threads.
use crate::Result;
use libloading::Library;
use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr,
    sync::OnceLock,
};
pub(crate) type Ptr = *mut c_void;
macro_rules! api {
    ($($field:ident:$ty:ty=$symbol:literal),+ $(,)?)=>{
        pub(crate) struct Api {$(pub(crate) $field:$ty,)+ _library:Library}
        impl Api {
            fn load()->Result<Self> {
                #[cfg(target_os="windows")] const NAMES:&[&str]=&["soft_oal.dll"];
                #[cfg(not(target_os="windows"))] const NAMES:&[&str]=&["libopenal.so.1"];
                // SAFETY: these are the published signatures. No device or
                // context outlives the process-global library kept by api().
                unsafe {let library=crate::libraries::load(NAMES)?;Ok(Self { $($field:*library.get(concat!($symbol,"\0").as_bytes()).map_err(|e|e.to_string())?,)+ _library:library})}
            }
        }
    }
}
api! {
    get_string:unsafe extern "C" fn(Ptr,i32)->*const c_char="alcGetString",
    open:unsafe extern "C" fn(*const c_char)->Ptr="alcOpenDevice",
    close:unsafe extern "C" fn(Ptr)->u8="alcCloseDevice",
    create:unsafe extern "C" fn(Ptr,*const i32)->Ptr="alcCreateContext",
    destroy:unsafe extern "C" fn(Ptr)="alcDestroyContext",
    set_current:unsafe extern "C" fn(Ptr)->u8="alcSetThreadContext",
    capture_open:unsafe extern "C" fn(*const c_char,u32,i32,i32)->Ptr="alcCaptureOpenDevice",
    capture_close:unsafe extern "C" fn(Ptr)->u8="alcCaptureCloseDevice",
    capture_start:unsafe extern "C" fn(Ptr)="alcCaptureStart",
    capture_stop:unsafe extern "C" fn(Ptr)="alcCaptureStop",
    capture_samples:unsafe extern "C" fn(Ptr,Ptr,i32)="alcCaptureSamples",
    integer:unsafe extern "C" fn(Ptr,i32,i32,*mut i32)="alcGetIntegerv",
    error:unsafe extern "C" fn(Ptr)->i32="alcGetError",
    al_error:unsafe extern "C" fn()->i32="alGetError",
    gen_sources:unsafe extern "C" fn(i32,*mut u32)="alGenSources",
    delete_sources:unsafe extern "C" fn(i32,*const u32)="alDeleteSources",
    gen_buffers:unsafe extern "C" fn(i32,*mut u32)="alGenBuffers",
    delete_buffers:unsafe extern "C" fn(i32,*const u32)="alDeleteBuffers",
    source_i:unsafe extern "C" fn(u32,i32,i32)="alSourcei",
    source_f:unsafe extern "C" fn(u32,i32,f32)="alSourcef",
    get_source_i:unsafe extern "C" fn(u32,i32,*mut i32)="alGetSourcei",
    data:unsafe extern "C" fn(u32,i32,*const c_void,i32,i32)="alBufferData",
    queue:unsafe extern "C" fn(u32,i32,*const u32)="alSourceQueueBuffers",
    unqueue:unsafe extern "C" fn(u32,i32,*mut u32)="alSourceUnqueueBuffers",
    play:unsafe extern "C" fn(u32)="alSourcePlay",
    stop:unsafe extern "C" fn(u32)="alSourceStop"
}
pub(crate) fn api() -> Result<&'static Api> {
    static API: OnceLock<Result<Api>> = OnceLock::new();
    API.get_or_init(Api::load).as_ref().map_err(Clone::clone)
}
pub(crate) struct Device {
    ptr: Ptr,
    capture: bool,
}
impl Api {
    pub(crate) fn devices(&self, input: bool) -> Vec<String> {
        let mut result = Vec::new();
        unsafe {
            let pointer = (self.get_string)(ptr::null_mut(), if input { 0x310 } else { 0x1013 });
            if pointer.is_null() {
                return result;
            }
            let mut offset = 0;
            while offset < 65536 && *pointer.add(offset) != 0 {
                let name = CStr::from_ptr(pointer.add(offset));
                offset += name.to_bytes_with_nul().len();
                result.push(name.to_string_lossy().into_owned());
                if result.len() >= 256 {
                    break;
                }
            }
        }
        result
    }
    pub(crate) fn current(&self, context: Ptr) -> Result<()> {
        if unsafe { (self.set_current)(context) } == 0 {
            Err("OpenAL could not select audio context".into())
        } else {
            Ok(())
        }
    }
    pub(crate) fn check(&self) -> Result<()> {
        let error = unsafe { (self.al_error)() };
        if error == 0 {
            Ok(())
        } else {
            Err(format!("OpenAL operation failed: {error:#x}"))
        }
    }
    pub(crate) fn capture(&self, name: &str, rate: u32, stereo: bool) -> Result<(Device, usize)> {
        let name = CString::new(name).map_err(|_| "invalid microphone name")?;
        let name = if name.as_bytes().is_empty() {
            ptr::null()
        } else {
            name.as_ptr()
        };
        let mut channels = if stereo { 2 } else { 1 };
        let mut device = unsafe {
            (self.capture_open)(
                name,
                rate,
                if stereo { 0x1103 } else { 0x1101 },
                rate as i32 / 4,
            )
        };
        if device.is_null() && stereo {
            channels = 1;
            device = unsafe { (self.capture_open)(name, rate, 0x1101, rate as i32 / 4) };
        }
        if device.is_null() {
            return Err("configured microphone is unavailable".into());
        }
        Ok((
            Device {
                ptr: device,
                capture: true,
            },
            channels,
        ))
    }
    pub(crate) fn output(&self, name: &str) -> Result<Device> {
        let name = CString::new(name).map_err(|_| "invalid speaker name")?;
        let device = unsafe {
            (self.open)(if name.as_bytes().is_empty() {
                ptr::null()
            } else {
                name.as_ptr()
            })
        };
        if device.is_null() {
            Err("configured output device is unavailable".into())
        } else {
            Ok(Device {
                ptr: device,
                capture: false,
            })
        }
    }
}
impl Device {
    pub(crate) fn context(&self) -> Result<Ptr> {
        let api = api()?;
        let attributes = [0x1007, 48000, 0];
        let context = unsafe { (api.create)(self.ptr, attributes.as_ptr()) };
        if context.is_null() {
            Err("OpenAL could not create playback context".into())
        } else {
            Ok(context)
        }
    }
    pub(crate) fn capture_active(&self, active: bool) {
        if let Ok(api) = api() {
            unsafe {
                if active {
                    (api.capture_start)(self.ptr)
                } else {
                    (api.capture_stop)(self.ptr)
                }
            }
        }
    }
    pub(crate) fn capture_read(&self, channels: usize) -> Vec<i16> {
        let Ok(api) = api() else { return vec![] };
        let mut count = 0;
        unsafe {
            (api.integer)(self.ptr, 0x312, 1, &mut count);
        }
        let count = count.clamp(0, 48000) as usize;
        let mut output = vec![0; count * channels];
        if count > 0 {
            unsafe {
                (api.capture_samples)(self.ptr, output.as_mut_ptr().cast(), count as i32);
            }
        }
        output
    }
    pub(crate) fn failed(&self) -> bool {
        api().map_or(true, |api| unsafe { (api.error)(self.ptr) } != 0)
    }
}
impl Drop for Device {
    fn drop(&mut self) {
        if let Ok(api) = api() {
            unsafe {
                if self.capture {
                    (api.capture_stop)(self.ptr);
                    (api.capture_close)(self.ptr);
                } else {
                    (api.close)(self.ptr);
                }
            }
        }
    }
}
