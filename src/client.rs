use std::io;
use std::net::SocketAddr;
use std::time::Instant;

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

        if res.len() < 4 {
            return Err(io::Error::other("Malformed CIP response for symbol browse"));
        }

        let general_status = res[2];
        if general_status != 0 {
            return Err(io::Error::other(format!(
                "PLC returned error 0x{:02X} for symbol browse",
                general_status
            )));
        }

        let ext_words = res[3] as usize;
        let data_start = 4 + ext_words * 2;
        if res.len() < data_start {
            return Err(io::Error::other("Symbol browse response too short"));
        }

        let symbols = parse_symbol_browse_response(&res[data_start..]);
        Ok(symbols)
    }

    pub async fn discover() -> io::Result<Vec<(String, String)>> {
        const ENIP_PORT: u16 = 44818;
        const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(1);
        const RECV_TIMEOUT: Duration = Duration::from_millis(200);
        const MIN_ENCAP_HEADER: usize = 24;
        const ETHERNET_IP_HEADER_SKIP: usize = 30;
        const IDENTITY_HEADER_LEN: usize = 32;

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;

        let msg = EncapsulationHeader::new(COMMAND_LIST_IDENTITY, 0, 0).to_bytes();
        socket
            .send_to(&msg, SocketAddr::from(([255, 255, 255, 255], ENIP_PORT)))
            .await?;

        let mut results = Vec::new();
        let mut buf = [0u8; 1024];
        let start = Instant::now();

        while start.elapsed() < DISCOVERY_TIMEOUT {
            if let Ok(Ok((len, addr))) = timeout(RECV_TIMEOUT, socket.recv_from(&mut buf)).await {
                if len < ETHERNET_IP_HEADER_SKIP + MIN_ENCAP_HEADER {
                    continue;
                }

                let data = &buf[..len];
                let payload = &data[ETHERNET_IP_HEADER_SKIP..];

                if payload.len() < IDENTITY_HEADER_LEN + 1 {
                    continue;
                }

                let name_len = payload[IDENTITY_HEADER_LEN] as usize;
                let name_start = IDENTITY_HEADER_LEN + 1;
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
        let hdr = EncapsulationHeader::from_bytes(&h_buf)
            .ok_or(io::Error::other("Handshake failed: invalid header"))?;

        if hdr.status != 0 {
            return Err(io::Error::other(format!(
                "RegisterSession failed with status 0x{:04X}",
                hdr.status
            )));
        }

        let mut s_buf = [0u8; 4];
        stream.read_exact(&mut s_buf).await?;

        Ok(Self {
            stream,
            session: hdr.session,
            slot: None,
        })
    }

    pub fn set_slot(&mut self, slot: u8) {
        if slot > 17 {
            return;
        }
        self.slot = Some(slot);
    }

    pub fn parse_cpf(data: &[u8]) -> io::Result<&[u8]> {
        if data.len() < 10 {
            return Err(io::Error::other("Data too short for CPF"));
        }

        let item_count = u16::from_le_bytes([data[6], data[7]]);
        let mut pos = 8;

        for _ in 0..item_count {
            if data.len() < pos + 4 {
                return Err(io::Error::other("CPF item header truncated"));
            }

            let type_id = u16::from_le_bytes([data[pos], data[pos + 1]]);
            let len = u16::from_le_bytes([data[pos + 2], data[pos + 3]]) as usize;
            pos += 4;

            if data.len() < pos + len {
                return Err(io::Error::other("CPF item length out of bounds"));
            }

            if type_id == 0x00B2 {
                return Ok(&data[pos..pos + len]);
            }

            pos += len;
        }

        Err(io::Error::other("No CIP data item found in CPF"))
    }

    pub async fn read_tag(&mut self, tag: &str) -> io::Result<CipValue> {
        let cip = build_read_request(tag, self.slot);

        self.send_rr_data(cip).await.and_then(|res| {
            if res.len() < 4 {
                return Err(io::Error::other("Malformed CIP read response"));
            }

            let general_status = res[2];
            if general_status != 0 {
                return Err(io::Error::other(format!(
                    "PLC returned error 0x{:02X}",
                    general_status
                )));
            }

            let ext_words = res[3] as usize;
            let data_start = 4 + ext_words * 2;
            if res.len() < data_start {
                return Err(io::Error::other("CIP read response too short"));
            }

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

    pub async fn read_tag_multi(&mut self, tag: &str, count: usize) -> io::Result<Vec<CipValue>> {
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let indexed = format!("{tag}[{i}]");
            out.push(self.read_tag(&indexed).await?);
        }
        Ok(out)
    }

    pub async fn write_tag_multi(&mut self, tag: &str, values: &[CipValue]) -> io::Result<()> {
        for (i, v) in values.iter().enumerate() {
            let indexed = format!("{tag}[{i}]");
            self.write_tag(&indexed, v.clone()).await?;
        }
        Ok(())
    }

    pub async fn read_tags_msp(&mut self, tags: &[&str]) -> io::Result<Vec<MultiResult<CipValue>>> {
        let mut reqs = Vec::with_capacity(tags.len());
        for tag in tags {
            let cip = build_read_request(tag, self.slot);
            reqs.push(cip);
        }

        let msp = build_cip_multiple_service_request(&reqs);
        let res = self.send_rr_data(msp).await?;
        Ok(parse_cip_multiple_service_response(&res))
    }

    async fn send_rr_data(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut rr = Vec::with_capacity(22 + cip.len());
        rr.extend_from_slice(&0u32.to_le_bytes()); // interface handle
        rr.extend_from_slice(&0u16.to_le_bytes()); // timeout
        rr.extend_from_slice(&2u16.to_le_bytes()); // item count

        rr.extend_from_slice(&0x0000u16.to_le_bytes());
        rr.extend_from_slice(&0u16.to_le_bytes());

        rr.extend_from_slice(&0x00B2u16.to_le_bytes());
        rr.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        rr.extend_from_slice(&cip);

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

            if h.length == 0 {
                return Err(io::Error::other("Empty encapsulation payload"));
            }

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
