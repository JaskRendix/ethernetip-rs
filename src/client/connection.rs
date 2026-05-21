use std::io;

use crate::cip::ConnectionParams;
use crate::encapsulation::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use super::{Connectable, ConnectionManagement, EthernetIpClient};

/// Shared helper for connect() and reconnect().
async fn open_session(ip: &str) -> io::Result<(TcpStream, u32)> {
    let mut stream = TcpStream::connect(format!("{ip}:44818")).await?;

    // Build RegisterSession packet
    let mut reg = EncapsulationHeader::new(COMMAND_REGISTER_SESSION, 4, 0)
        .to_bytes()
        .to_vec();
    reg.extend_from_slice(&1u16.to_le_bytes());
    reg.extend_from_slice(&0u16.to_le_bytes());

    stream.write_all(&reg).await?;

    // Read 24‑byte header
    let mut h_buf = [0u8; 24];
    stream.read_exact(&mut h_buf).await?;
    let hdr = EncapsulationHeader::from_bytes(&h_buf)
        .ok_or(io::Error::other("Handshake failed: invalid header"))?;

    if hdr.status != 0 {
        return Err(io::Error::other(format!(
            "RegisterSession failed with status 0x{:04X}",
            hdr.status
        )));
    }

    // Read protocol version + options
    let mut s_buf = [0u8; 4];
    stream.read_exact(&mut s_buf).await?;

    Ok((stream, hdr.session))
}

#[async_trait::async_trait]
impl Connectable for EthernetIpClient {
    async fn connect(ip: &str) -> io::Result<Self> {
        let (stream, session) = open_session(ip).await?;

        Ok(Self {
            stream,
            session,
            slot: None,
            connection_id: None,
            sequence: 1,
            retries: 3,
            ip: ip.to_string(),
            connected: false,
            connection_params: ConnectionParams::default(),
        })
    }

    async fn close(mut self) -> io::Result<()> {
        let pkt = EncapsulationHeader::new(COMMAND_UNREGISTER_SESSION, 0, self.session).to_bytes();
        self.stream.write_all(&pkt).await?;
        Ok(())
    }
}

impl ConnectionManagement for EthernetIpClient {
    fn set_slot(&mut self, slot: u8) {
        if slot > 17 {
            return;
        }
        self.slot = Some(slot);
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn sequence(&self) -> u16 {
        self.sequence
    }

    fn set_retries(&mut self, retries: usize) {
        self.retries = retries;
    }

    fn connection_ip(&self) -> &str {
        &self.ip
    }
}

impl EthernetIpClient {
    pub(crate) async fn reconnect(&mut self) -> io::Result<()> {
        let (stream, session) = open_session(&self.ip).await?;
        self.stream = stream;
        self.session = session;
        Ok(())
    }
}
