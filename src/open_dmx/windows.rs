use crate::model::Result;
use std::ffi::c_void;
type Handle = *mut c_void;
#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryExW(name: *const u16, file: Handle, flags: u32) -> Handle;
    fn GetProcAddress(module: Handle, name: *const u8) -> *mut c_void;
    fn FreeLibrary(module: Handle) -> i32;
}
pub(super) struct Library(Handle);
impl Library {
    pub fn load() -> Result<Self> {
        // LOAD_LIBRARY_SEARCH_SYSTEM32 excludes project/current-directory DLLs.
        let name: Vec<u16> = "ftd2xx.dll\0".encode_utf16().collect();
        let raw = unsafe { LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), 0x800) };
        if raw.is_null() {
            Err("FTDI D2XX driver unavailable. Install the official FTDI Windows driver".into())
        } else {
            Ok(Self(raw))
        }
    }
    pub fn symbol(&self, name: &[u8]) -> Result<*mut c_void> {
        let raw = unsafe { GetProcAddress(self.0, name.as_ptr()) };
        if raw.is_null() {
            Err("Required FTDI driver symbol unavailable".into())
        } else {
            Ok(raw)
        }
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}
