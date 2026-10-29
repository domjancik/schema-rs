use crate::model::Result;
use std::ffi::{c_char, c_void, CStr, CString};
#[link(name = "System")]
unsafe extern "C" {
    fn dlopen(path: *const c_char, mode: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
    fn dlerror() -> *const c_char;
}
pub(super) struct Library(*mut c_void);
impl Library {
    pub fn load() -> Result<Self> {
        let mut paths = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(contents) = exe
                .parent()
                .and_then(|p| p.parent())
                .filter(|p| p.file_name().is_some_and(|n| n == "Contents"))
            {
                paths.push(contents.join("Frameworks/libftd2xx.dylib"));
            }
        }
        paths.extend(
            [
                "/usr/local/lib/libftd2xx.dylib",
                "/opt/homebrew/lib/libftd2xx.dylib",
            ]
            .map(std::path::PathBuf::from),
        );
        let mut errors = Vec::new();
        for path in paths {
            if !path.is_file() {
                continue;
            }
            let name = CString::new(path.as_os_str().as_encoded_bytes())
                .map_err(|_| "Invalid FTDI library path")?;
            let handle = unsafe { dlopen(name.as_ptr(), 2 | 4) };
            if !handle.is_null() {
                return Ok(Self(handle));
            }
            let message = unsafe { dlerror() };
            if !message.is_null() {
                errors.push(
                    unsafe { CStr::from_ptr(message) }
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        Err(format!("FTDI D2XX unavailable on macOS. Install the matching official FTDI library or use the signed app with bundled D2XX. {}",errors.join("; ")))
    }
    pub fn symbol(&self, name: &[u8]) -> Result<*mut c_void> {
        let ptr = unsafe { dlsym(self.0, name.as_ptr().cast()) };
        if ptr.is_null() {
            Err("Required FTDI D2XX macOS symbol unavailable".into())
        } else {
            Ok(ptr)
        }
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            dlclose(self.0);
        }
    }
}
