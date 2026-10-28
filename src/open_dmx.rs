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
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{devices, Sender};
#[cfg(not(windows))]
pub fn devices() -> Result<Vec<Device>> {
    Err("Open DMX USB is available on Windows; use Art-Net/OLA on Linux".into())
}
#[cfg(not(windows))]
pub struct Sender;
#[cfg(not(windows))]
impl Sender {
    pub fn open(_: &str) -> Result<Self> {
        Err("Open DMX USB requires Windows FTDI D2XX".into())
    }
    pub fn send(&mut self, _: &[u8]) -> Result<()> {
        Err("Open DMX USB requires Windows FTDI D2XX".into())
    }
}
