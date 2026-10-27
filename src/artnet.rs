use crate::model::Result;
pub fn packet(universe: u16, sequence: u8, channels: &[u8]) -> Result<Vec<u8>> {
    if universe > 32767 || channels.len() > 512 {
        return Err("Invalid ArtDmx universe or payload".into());
    }
    let length = channels.len().max(2).next_multiple_of(2);
    let mut bytes = Vec::with_capacity(18 + length);
    bytes.extend_from_slice(b"Art-Net\0");
    bytes.extend_from_slice(&[0, 0x50, 0, 14, sequence, 0]);
    bytes.extend_from_slice(&universe.to_le_bytes());
    bytes.extend_from_slice(&(length as u16).to_be_bytes());
    bytes.extend_from_slice(channels);
    bytes.resize(18 + length, 0);
    Ok(bytes)
}
pub struct Sender {
    socket: std::net::UdpSocket,
    sequence: u8,
}
impl Sender {
    pub fn new() -> Result<Self> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Self {
            socket,
            sequence: 0,
        })
    }
    pub fn send(
        &mut self,
        target: std::net::SocketAddr,
        universes: &std::collections::BTreeMap<u16, Vec<u8>>,
    ) -> Result<()> {
        self.sequence = if self.sequence == 255 {
            1
        } else {
            self.sequence + 1
        };
        for (universe, values) in universes {
            let data = packet(*universe, self.sequence, values)?;
            self.socket
                .send_to(&data, target)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}
