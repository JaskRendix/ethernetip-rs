use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::cip::{
    build_cip_multiple_service_request, build_cip_request, build_cip_request_with_slot,
    build_cip_write_array, build_cip_write_fragment, parse_cip_multiple_service_response,
    CipService,
};
use crate::encapsulation::{EncapsulationHeader, COMMAND_REGISTER_SESSION, COMMAND_SEND_RR_DATA};
use crate::types::{CipValue, MultiResult};

pub struct EthernetIpClient {
    stream: TcpStream,
    session: u32,
    slot: Option<u8>,
}

impl EthernetIpClient {
    pub async fn connect(host: &str, port: u16) -> io::Result<Self> {
        let addr = format!("{}:{}", host, port);
        let mut stream = TcpStream::connect(addr).await?;

        let session = register_session(&mut stream).await?;
        Ok(Self {
            stream,
            session,
            slot: None,
        })
    }

    pub fn set_slot(&mut self, slot: u8) {
        self.slot = Some(slot);
    }

    pub async fn read_tag(&mut self, tag: &str) -> io::Result<Option<CipValue>> {
        let cip = if let Some(slot) = self.slot {
            build_cip_request_with_slot(CipService::ReadData, tag, Some(slot), None, Some(1))
        } else {
            build_cip_request(CipService::ReadData, tag, None, Some(1))
        };

        let mut cmd_data = Vec::new();
        cmd_data.extend_from_slice(&0u32.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&2u16.to_le_bytes());

        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());

        cmd_data.extend_from_slice(&0x00B2u16.to_le_bytes());
        cmd_data.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        cmd_data.extend_from_slice(&cip);

        let header =
            EncapsulationHeader::new(COMMAND_SEND_RR_DATA, cmd_data.len() as u16, self.session);
        let mut packet = Vec::with_capacity(EncapsulationHeader::SIZE + cmd_data.len());
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(&cmd_data);

        self.stream.write_all(&packet).await?;

        let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
        self.stream.read_exact(&mut hdr_buf).await?;
        let hdr = EncapsulationHeader::from_bytes(&hdr_buf).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid encapsulation header")
        })?;

        if hdr.length == 0 {
            return Ok(None);
        }

        let mut data = vec![0u8; hdr.length as usize];
        self.stream.read_exact(&mut data).await?;

        if data.len() < 4 + 2 + 2 + 4 + 4 {
            return Ok(None);
        }
        let cip_offset = 4 + 2 + 2 + 4 + 4;
        if data.len() <= cip_offset {
            return Ok(None);
        }
        let cip_payload = &data[cip_offset..];

        if cip_payload.len() < 2 {
            return Ok(None);
        }

        let path_size_words = cip_payload[1] as usize;
        let path_bytes = path_size_words * 2;
        let data_offset = 2 + path_bytes;
        if cip_payload.len() <= data_offset {
            return Ok(None);
        }
        let value_bytes = &cip_payload[data_offset..];

        Ok(crate::cip::decode_cip_response(value_bytes))
    }

    pub async fn write_tag(&mut self, tag: &str, value: CipValue) -> io::Result<()> {
        let cip = if let Some(slot) = self.slot {
            build_cip_request_with_slot(
                CipService::WriteData,
                tag,
                Some(slot),
                Some(&value),
                Some(1),
            )
        } else {
            build_cip_request(CipService::WriteData, tag, Some(&value), Some(1))
        };

        let mut cmd_data = Vec::new();
        cmd_data.extend_from_slice(&0u32.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&2u16.to_le_bytes());

        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());

        cmd_data.extend_from_slice(&0x00B2u16.to_le_bytes());
        cmd_data.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        cmd_data.extend_from_slice(&cip);

        let header =
            EncapsulationHeader::new(COMMAND_SEND_RR_DATA, cmd_data.len() as u16, self.session);

        let mut packet = Vec::new();
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(&cmd_data);

        self.stream.write_all(&packet).await?;

        let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
        self.stream.read_exact(&mut hdr_buf).await?;
        let hdr = EncapsulationHeader::from_bytes(&hdr_buf)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid header"))?;

        let mut data = vec![0u8; hdr.length as usize];
        self.stream.read_exact(&mut data).await?;

        let cip_offset = 4 + 2 + 2 + 4 + 4;
        if data.len() < cip_offset + 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "CIP response too short",
            ));
        }

        let path_size_words = data[cip_offset + 1] as usize;
        let path_bytes = path_size_words * 2;
        let status_offset = cip_offset + 2 + path_bytes;

        let general_status = data[status_offset];
        if general_status != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("CIP write failed with status 0x{:02X}", general_status),
            ));
        }

        Ok(())
    }

    pub async fn write_array_tag(&mut self, tag: &str, value: CipValue) -> io::Result<()> {
        let cip = build_cip_write_array(tag, &value);

        let mut cmd_data = Vec::new();
        cmd_data.extend_from_slice(&0u32.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&2u16.to_le_bytes());

        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());

        cmd_data.extend_from_slice(&0x00B2u16.to_le_bytes());
        cmd_data.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        cmd_data.extend_from_slice(&cip);

        let header =
            EncapsulationHeader::new(COMMAND_SEND_RR_DATA, cmd_data.len() as u16, self.session);

        let mut packet = Vec::new();
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(&cmd_data);

        self.stream.write_all(&packet).await?;

        let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
        self.stream.read_exact(&mut hdr_buf).await?;
        let hdr = EncapsulationHeader::from_bytes(&hdr_buf)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid header"))?;

        let mut data = vec![0u8; hdr.length as usize];
        self.stream.read_exact(&mut data).await?;

        let cip_offset = 4 + 2 + 2 + 4 + 4;
        let path_size_words = data[cip_offset + 1] as usize;
        let path_bytes = path_size_words * 2;
        let status_offset = cip_offset + 2 + path_bytes;

        let general_status = data[status_offset];
        if general_status != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("CIP write failed with status 0x{:02X}", general_status),
            ));
        }

        Ok(())
    }

    pub async fn write_array_fragment(
        &mut self,
        tag: &str,
        value: CipValue,
        array_size: u16,
        start_index: u32,
        write_count: u16,
    ) -> io::Result<()> {
        let cip = build_cip_write_fragment(tag, &value, array_size, start_index, write_count);

        let mut cmd_data = Vec::new();
        cmd_data.extend_from_slice(&0u32.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&2u16.to_le_bytes());

        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());

        cmd_data.extend_from_slice(&0x00B2u16.to_le_bytes());
        cmd_data.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        cmd_data.extend_from_slice(&cip);

        let header =
            EncapsulationHeader::new(COMMAND_SEND_RR_DATA, cmd_data.len() as u16, self.session);

        let mut packet = Vec::new();
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(&cmd_data);

        self.stream.write_all(&packet).await?;

        let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
        self.stream.read_exact(&mut hdr_buf).await?;
        let hdr = EncapsulationHeader::from_bytes(&hdr_buf)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid header"))?;

        let mut data = vec![0u8; hdr.length as usize];
        self.stream.read_exact(&mut data).await?;

        let cip_offset = 4 + 2 + 2 + 4 + 4;
        let path_size_words = data[cip_offset + 1] as usize;
        let path_bytes = path_size_words * 2;
        let status_offset = cip_offset + 2 + path_bytes;

        let general_status = data[status_offset];
        if general_status != 0 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "CIP write fragment failed with status 0x{:02X}",
                    general_status
                ),
            ));
        }

        Ok(())
    }

    pub async fn read_tags_multi(&mut self, tags: &[&str]) -> io::Result<Vec<MultiResult>> {
        let mut requests = Vec::new();
        for tag in tags {
            let cip = if let Some(slot) = self.slot {
                build_cip_request_with_slot(CipService::ReadData, tag, Some(slot), None, Some(1))
            } else {
                build_cip_request(CipService::ReadData, tag, None, Some(1))
            };
            requests.push(cip);
        }

        let msp = build_cip_multiple_service_request(&requests);

        let mut cmd_data = Vec::new();
        cmd_data.extend_from_slice(&0u32.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&2u16.to_le_bytes());

        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());

        cmd_data.extend_from_slice(&0x00B2u16.to_le_bytes());
        cmd_data.extend_from_slice(&(msp.len() as u16).to_le_bytes());
        cmd_data.extend_from_slice(&msp);

        let header =
            EncapsulationHeader::new(COMMAND_SEND_RR_DATA, cmd_data.len() as u16, self.session);

        let mut packet = Vec::new();
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(&cmd_data);

        self.stream.write_all(&packet).await?;

        let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
        self.stream.read_exact(&mut hdr_buf).await?;
        let hdr = EncapsulationHeader::from_bytes(&hdr_buf)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid header"))?;

        let mut data = vec![0u8; hdr.length as usize];
        self.stream.read_exact(&mut data).await?;

        let cip_offset = 4 + 2 + 2 + 4 + 4;
        let msp_response = &data[cip_offset..];

        Ok(parse_cip_multiple_service_response(msp_response))
    }

    pub async fn write_tags_multi(
        &mut self,
        tags_and_values: &[(&str, CipValue)],
    ) -> io::Result<Vec<MultiResult>> {
        let mut requests = Vec::new();
        for (tag, value) in tags_and_values {
            let cip = if let Some(slot) = self.slot {
                build_cip_request_with_slot(
                    CipService::WriteData,
                    tag,
                    Some(slot),
                    Some(value),
                    Some(1),
                )
            } else {
                build_cip_request(CipService::WriteData, tag, Some(value), Some(1))
            };
            requests.push(cip);
        }

        let msp = build_cip_multiple_service_request(&requests);

        let mut cmd_data = Vec::new();
        cmd_data.extend_from_slice(&0u32.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&2u16.to_le_bytes());

        cmd_data.extend_from_slice(&0u16.to_le_bytes());
        cmd_data.extend_from_slice(&0u16.to_le_bytes());

        cmd_data.extend_from_slice(&0x00B2u16.to_le_bytes());
        cmd_data.extend_from_slice(&(msp.len() as u16).to_le_bytes());
        cmd_data.extend_from_slice(&msp);

        let header =
            EncapsulationHeader::new(COMMAND_SEND_RR_DATA, cmd_data.len() as u16, self.session);

        let mut packet = Vec::new();
        packet.extend_from_slice(&header.to_bytes());
        packet.extend_from_slice(&cmd_data);

        self.stream.write_all(&packet).await?;

        let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
        self.stream.read_exact(&mut hdr_buf).await?;
        let hdr = EncapsulationHeader::from_bytes(&hdr_buf)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid header"))?;

        let mut data = vec![0u8; hdr.length as usize];
        self.stream.read_exact(&mut data).await?;

        let cip_offset = 4 + 2 + 2 + 4 + 4;
        let msp_response = &data[cip_offset..];

        Ok(parse_cip_multiple_service_response(msp_response))
    }
}

async fn register_session(stream: &mut TcpStream) -> io::Result<u32> {
    let mut cmd_data = Vec::new();
    cmd_data.extend_from_slice(&1u16.to_le_bytes());
    cmd_data.extend_from_slice(&0u16.to_le_bytes());

    let header = EncapsulationHeader::new(COMMAND_REGISTER_SESSION, cmd_data.len() as u16, 0);
    let mut packet = Vec::with_capacity(EncapsulationHeader::SIZE + cmd_data.len());
    packet.extend_from_slice(&header.to_bytes());
    packet.extend_from_slice(&cmd_data);

    stream.write_all(&packet).await?;

    let mut hdr_buf = [0u8; EncapsulationHeader::SIZE];
    stream.read_exact(&mut hdr_buf).await?;
    let hdr = EncapsulationHeader::from_bytes(&hdr_buf).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid register session header",
        )
    })?;

    if hdr.length < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Register session response too short",
        ));
    }

    let mut data = vec![0u8; hdr.length as usize];
    stream.read_exact(&mut data).await?;

    if data.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Register session data too short",
        ));
    }
    let session = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    Ok(session)
}
