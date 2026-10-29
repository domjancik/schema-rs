//! One unbuffered Open DMX USB universe. Only the Windows adapter links to the OS.
use crate::model::Result;
use std::time::Duration;
#[derive(Clone, Debug, serde::Serialize)]
pub struct Device {
    pub serial: String,
    pub description: String,
    pub device_type: u32,
}
pub fn frame(channels: &[u8]) -> Result<[u8; 513]> {
    if channels.len() > 512 {
        return Err("Open DMX supports at most512 channels".into());
    }
    let mut bytes = [0; 513];
    bytes[1..channels.len() + 1].copy_from_slice(channels);
    Ok(bytes)
}
pub trait Wire {
    fn break_on(&mut self) -> Result<()>;
    fn break_off(&mut self) -> Result<()>;
    fn write(&mut self, bytes: &[u8]) -> Result<usize>;
}
/// Host-controlled BREAK/MAB. A short write invalidates the frame; never send a partial retry.
pub fn transmit(
    wire: &mut impl Wire,
    bytes: &[u8; 513],
    mut wait: impl FnMut(Duration),
) -> Result<()> {
    wire.break_on()?;
    wait(Duration::from_micros(120));
    wire.break_off()?;
    wait(Duration::from_micros(16));
    if wire.write(bytes)? != bytes.len() {
        return Err("Open DMX short USB write".into());
    }
    Ok(())
}
pub const fn supported() -> bool {
    cfg!(any(windows, target_os = "macos"))
}
#[cfg(any(windows, target_os = "macos"))]
mod d2xx;
#[cfg(windows)]
#[path = "open_dmx/windows.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "open_dmx/macos.rs"]
mod platform;
#[cfg(any(windows, target_os = "macos"))]
pub use d2xx::{devices, Sender};
#[cfg(not(any(windows, target_os = "macos")))]
pub fn devices() -> Result<Vec<Device>> {
    Err("Open DMX USB is available on Windows/macOS; use Art-Net/OLA on Linux".into())
}
#[cfg(not(any(windows, target_os = "macos")))]
pub struct Sender;
#[cfg(not(any(windows, target_os = "macos")))]
impl Sender {
    pub fn open(_: &str) -> Result<Self> {
        Err("Open DMX USB requires Windows or macOS FTDI D2XX".into())
    }
    pub fn send(&mut self, _: &[u8]) -> Result<()> {
        Err("Open DMX USB requires Windows or macOS FTDI D2XX".into())
    }
}
