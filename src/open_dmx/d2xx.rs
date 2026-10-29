// D2XX macOS WinTypes.h defines DWORD/ULONG/FT_STATUS as unsigned int (32-bit).
// extern "system" is Win32 ABI on Windows and C ABI on macOS.
use super::{platform::Library, Device, Wire};
use crate::model::Result;
use std::{
    ffi::{c_void, CString},
    time::{Duration, Instant},
};
type Handle = *mut c_void;
type Status = u32;
type One = unsafe extern "system" fn(Handle) -> Status;
type U32 = unsafe extern "system" fn(Handle, u32) -> Status;
type Byte = unsafe extern "system" fn(Handle, u8) -> Status;
type Data = unsafe extern "system" fn(Handle, u8, u8, u8) -> Status;
type Flow = unsafe extern "system" fn(Handle, u16, u8, u8) -> Status;
type Timeouts = unsafe extern "system" fn(Handle, u32, u32) -> Status;
type Write = unsafe extern "system" fn(Handle, *const u8, u32, *mut u32) -> Status;
type GetStatus = unsafe extern "system" fn(Handle, *mut u32, *mut u32, *mut u32) -> Status;
type Open = unsafe extern "system" fn(*const c_void, u32, *mut Handle) -> Status;
type Count = unsafe extern "system" fn(*mut u32) -> Status;
type Info = unsafe extern "system" fn(
    u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u32,
    *mut u8,
    *mut u8,
    *mut Handle,
) -> Status;
struct Api {
    _library: Library,
    open: Open,
    close: One,
    reset: One,
    baud: U32,
    data: Data,
    flow: Flow,
    timeouts: Timeouts,
    latency: Byte,
    purge: U32,
    on: One,
    off: One,
    write: Write,
    status: GetStatus,
    count: Count,
    info: Info,
}
fn check(code: Status, op: &str) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!(
            "FTDI {op} failed (status{code}); check connection and exclusive device access"
        ))
    }
}
impl Api {
    fn load() -> Result<Self> {
        let library = Library::load()?;
        macro_rules! symbol {
            ($name:literal,$ty:ty) => {{
                let raw = library.symbol(concat!($name, "\0").as_bytes())?;
                if raw.is_null() {
                    return Err(concat!("FTDI driver missing ", $name).into());
                }
                unsafe { std::mem::transmute::<*mut c_void, $ty>(raw) }
            }};
        }
        Ok(Self {
            open: symbol!("FT_OpenEx", Open),
            close: symbol!("FT_Close", One),
            reset: symbol!("FT_ResetDevice", One),
            baud: symbol!("FT_SetBaudRate", U32),
            data: symbol!("FT_SetDataCharacteristics", Data),
            flow: symbol!("FT_SetFlowControl", Flow),
            timeouts: symbol!("FT_SetTimeouts", Timeouts),
            latency: symbol!("FT_SetLatencyTimer", Byte),
            purge: symbol!("FT_Purge", U32),
            on: symbol!("FT_SetBreakOn", One),
            off: symbol!("FT_SetBreakOff", One),
            write: symbol!("FT_Write", Write),
            status: symbol!("FT_GetStatus", GetStatus),
            count: symbol!("FT_CreateDeviceInfoList", Count),
            info: symbol!("FT_GetDeviceInfoDetail", Info),
            _library: library,
        })
    }
    fn devices(&self) -> Result<Vec<Device>> {
        let mut count = 0;
        check(unsafe { (self.count)(&mut count) }, "enumerate")?;
        if count > 256 {
            return Err("Too many FTDI devices".into());
        }
        let mut found = Vec::new();
        for i in 0..count {
            let (mut flags, mut kind, mut id, mut location) = (0, 0, 0, 0);
            let (mut serial, mut description) = ([0u8; 16], [0u8; 64]);
            let mut handle = std::ptr::null_mut();
            check(
                unsafe {
                    (self.info)(
                        i,
                        &mut flags,
                        &mut kind,
                        &mut id,
                        &mut location,
                        serial.as_mut_ptr(),
                        description.as_mut_ptr(),
                        &mut handle,
                    )
                },
                "device info",
            )?;
            let text = |bytes: &[u8]| {
                String::from_utf8_lossy(
                    &bytes[..bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len())],
                )
                .chars()
                .filter(|c| !c.is_control())
                .collect::<String>()
            };
            let serial = text(&serial);
            if !serial.is_empty() {
                found.push(Device {
                    serial,
                    description: text(&description),
                    device_type: kind,
                });
            }
        }
        Ok(found)
    }
}
pub fn devices() -> Result<Vec<Device>> {
    Api::load()?.devices()
}
pub struct Sender {
    api: Api,
    handle: Handle,
}
impl Sender {
    pub fn open(serial: &str) -> Result<Self> {
        let api = Api::load()?;
        if api.devices()?.iter().filter(|d| d.serial == serial).count() != 1 {
            return Err("Select a unique connected Open DMX FTDI serial number".into());
        }
        let name = CString::new(serial).map_err(|_| "Invalid USB serial number")?;
        let mut handle = std::ptr::null_mut();
        check(
            unsafe { (api.open)(name.as_ptr().cast(), 1, &mut handle) },
            "open",
        )?;
        let sender = Self { api, handle };
        let a = &sender.api;
        check(unsafe { (a.reset)(handle) }, "reset")?;
        check(unsafe { (a.baud)(handle, 250000) }, "250k baud")?;
        check(unsafe { (a.data)(handle, 8, 2, 0) }, "8N2 format")?;
        check(unsafe { (a.flow)(handle, 0, 0, 0) }, "flow control")?;
        check(unsafe { (a.timeouts)(handle, 0, 250) }, "timeouts")?;
        check(unsafe { (a.latency)(handle, 2) }, "latency")?;
        check(unsafe { (a.purge)(handle, 3) }, "purge")?;
        check(unsafe { (a.off)(handle) }, "break off")?;
        Ok(sender)
    }
    pub fn send(&mut self, channels: &[u8]) -> Result<()> {
        let bytes = super::frame(channels)?;
        super::transmit(self, &bytes, |d| {
            let until = Instant::now() + d;
            while Instant::now() < until {
                std::hint::spin_loop();
            }
        })?;
        let deadline = Instant::now() + Duration::from_millis(250);
        loop {
            let (mut rx, mut tx, mut events) = (0, 0, 0);
            check(
                unsafe { (self.api.status)(self.handle, &mut rx, &mut tx, &mut events) },
                "TX queue",
            )?;
            if tx == 0 {
                break;
            }
            if Instant::now() >= deadline {
                return Err("FTDI transmit queue timed out".into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // The unbuffered adapter must finish all513 bytes (22.572ms at250k/8N2) before another BREAK.
        std::thread::sleep(Duration::from_millis(24));
        Ok(())
    }
}
impl Wire for Sender {
    fn break_on(&mut self) -> Result<()> {
        check(unsafe { (self.api.on)(self.handle) }, "break on")
    }
    fn break_off(&mut self) -> Result<()> {
        check(unsafe { (self.api.off)(self.handle) }, "break off")
    }
    fn write(&mut self, bytes: &[u8]) -> Result<usize> {
        let mut written = 0;
        check(
            unsafe {
                (self.api.write)(
                    self.handle,
                    bytes.as_ptr(),
                    bytes.len() as u32,
                    &mut written,
                )
            },
            "write",
        )?;
        Ok(written as usize)
    }
}
impl Drop for Sender {
    fn drop(&mut self) {
        unsafe {
            (self.api.off)(self.handle);
            (self.api.close)(self.handle);
        }
    }
}
