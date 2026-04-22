use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::{timeout, Duration};

use crate::cip::*;
use crate::encapsulation::*;
use crate::types::*;

pub struct EthernetIpClient {
    stream: TcpStream,
    session: u32,
    slot: Option<u8>,
}

impl EthernetIpClient {
    pub async fn browse_symbols(&mut self) -> io::Result<Vec<SymbolInfo>> {
        let cip = build_symbol_browse_request();
        let res = self.send_rr_data(cip).await?;

        let general_status = res[2];
        if general_status != 0 {
            return Err(io::Error::other(format!(
                "PLC returned error 0x{:02X} for symbol browse",
                general_status
            )));
        }

        let ext_words = res[3] as usize;
        let data_start = 4 + ext_words * 2;

        let symbols = parse_symbol_browse_response(&res[data_start..]);
        Ok(symbols)
    }

    pub async fn discover() -> io::Result<Vec<(String, String)>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;

        let msg = EncapsulationHeader::new(COMMAND_LIST_IDENTITY, 0, 0).to_bytes();
        socket.send_to(&msg, "255.255.255.255:44818").await?;

        let mut results = Vec::new();
        let mut buf = [0u8; 1024];
        let start = std::time::Instant::now();

        while start.elapsed() < Duration::from_secs(1) {
            if let Ok(Ok((len, addr))) =
                timeout(Duration::from_millis(200), socket.recv_from(&mut buf)).await
            {
                let data = &buf[..len];

                if data.len() < 30 {
                    continue;
                }

                let payload = &data[30..];

                let name_offset = 2 + 16 + 2 + 2 + 2 + 2 + 2 + 4; // = 32
                if payload.len() < name_offset + 1 {
                    continue;
                }

                let name_len = payload[name_offset] as usize;
                let name_start = name_offset + 1;
                if payload.len() < name_start + name_len {
                    continue;
                }

                let name = String::from_utf8_lossy(&payload[name_start..name_start + name_len])
                    .into_owned();
                results.push((addr.ip().to_string(), name));
            }
        }

        Ok(results)
    }

    pub async fn connect(ip: &str) -> io::Result<Self> {
        let mut stream = TcpStream::connect(format!("{}:44818", ip)).await?;

        let mut reg = EncapsulationHeader::new(COMMAND_REGISTER_SESSION, 4, 0)
            .to_bytes()
            .to_vec();
        reg.extend_from_slice(&1u16.to_le_bytes());
        reg.extend_from_slice(&0u16.to_le_bytes());

        stream.write_all(&reg).await?;

        let mut h_buf = [0u8; 24];
        stream.read_exact(&mut h_buf).await?;
        let hdr =
            EncapsulationHeader::from_bytes(&h_buf).ok_or(io::Error::other("Handshake failed"))?;

        let mut s_buf = [0u8; 4];
        stream.read_exact(&mut s_buf).await?;

        Ok(Self {
            stream,
            session: hdr.session,
            slot: None, // missing
        })
    }

    pub fn set_slot(&mut self, slot: u8) {
        self.slot = Some(slot);
    }

    pub fn parse_cpf(data: &[u8]) -> io::Result<&[u8]> {
        if data.len() < 10 {
            return Err(io::Error::other("Data too short"));
        }

        let item_count = u16::from_le_bytes([data[6], data[7]]);
        let mut pos = 8;

        for _ in 0..item_count {
            if data.len() < pos + 4 {
                break;
            }

            let type_id = u16::from_le_bytes([data[pos], data[pos + 1]]);
            let len = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as usize;
            pos += 4;

            if type_id == 0x00B2 {
                return Ok(&data[pos..pos + len]);
            }

            pos += len;
        }

        Err(io::Error::other("No CIP data item found"))
    }

    pub async fn read_tag(&mut self, tag: &str) -> io::Result<CipValue> {
        let cip = build_read_request(tag, self.slot);

        self.send_rr_data(cip).await.and_then(|res| {
            let general_status = res[2];
            if general_status != 0 {
                return Err(io::Error::other(format!(
                    "PLC returned error 0x{:02X}",
                    general_status
                )));
            }

            let ext_words = res[3] as usize;
            let data_start = 4 + ext_words * 2;

            decode_cip_response(&res[data_start..]).ok_or(io::Error::other("Decode error"))
        })
    }

    pub async fn write_tag(&mut self, tag: &str, value: CipValue) -> io::Result<()> {
        let cip = build_write_request(tag, &value, self.slot);

        let res = self.send_rr_data(cip).await?;

        decode_write_response(&res).map_err(|status| {
            io::Error::other(format!("PLC returned write error 0x{:02X}", status))
        })
    }

    async fn send_rr_data(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut rr = Vec::new();
        rr.extend_from_slice(&0u32.to_le_bytes());
        rr.extend_from_slice(&0u16.to_le_bytes());
        rr.extend_from_slice(&2u16.to_le_bytes());
        rr.extend_from_slice(&0x0000u16.to_le_bytes());
        rr.extend_from_slice(&0u16.to_le_bytes());
        rr.extend_from_slice(&0x00B2u16.to_le_bytes());
        rr.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        rr.extend(cip);

        let pkt = EncapsulationHeader::new(COMMAND_SEND_RR_DATA, rr.len() as u16, self.session)
            .to_bytes()
            .to_vec();

        timeout(Duration::from_secs(3), async {
            self.stream.write_all(&pkt).await?;
            self.stream.write_all(&rr).await?;

            let mut h_buf = [0u8; 24];
            self.stream.read_exact(&mut h_buf).await?;
            let h = EncapsulationHeader::from_bytes(&h_buf)
                .ok_or_else(|| io::Error::other("Bad encapsulation header"))?;

            let mut d = vec![0u8; h.length as usize];
            self.stream.read_exact(&mut d).await?;

            Ok(Self::parse_cpf(&d)?.to_vec())
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "PLC Timeout"))?
    }

    pub async fn close(mut self) -> io::Result<()> {
        let pkt = EncapsulationHeader::new(COMMAND_UNREGISTER_SESSION, 0, self.session).to_bytes();
        self.stream.write_all(&pkt).await?;
        Ok(())
    }
}
