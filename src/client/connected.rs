use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cip::{
    build_forward_close_request, build_forward_open_request, build_large_forward_close_request,
    build_large_forward_open_request, decode_extended_status, map_extended_status,
    ForwardOpenError,
};
use crate::client::UnconnectedMessaging;
use crate::encapsulation::*;

use super::{ConnectedMessaging, EthernetIpClient};

#[async_trait::async_trait]
impl ConnectedMessaging for EthernetIpClient {
    async fn forward_open(&mut self) -> Result<(), ForwardOpenError> {
        let cip = build_forward_open_request(self.slot, self.connection_params);

        let res = if self.connected {
            self.send_unit_data(cip).await?
        } else {
            self.send_rr_data(cip).await?
        };

        if res.len() < 10 {
            return Err(ForwardOpenError::Other(
                "ForwardOpen response too short".into(),
            ));
        }

        let status = res[2];
        if status != 0 {
            let ext = decode_extended_status(&res);

            if !ext.is_empty() {
                return Err(map_extended_status(&ext));
            }

            return Err(ForwardOpenError::GeneralStatus(status));
        }

        let conn_id = u32::from_le_bytes([res[6], res[7], res[8], res[9]]);
        self.connection_id = Some(conn_id);
        self.sequence = 1;
        self.connected = true;

        Ok(())
    }

    async fn forward_close(&mut self) -> io::Result<()> {
        if self.connection_id.is_none() {
            return Ok(());
        }

        let cip = build_forward_close_request(self.slot);

        if self.connected {
            let _ = self.send_unit_data(cip).await;
        } else {
            let _ = self.send_rr_data(cip).await;
        }

        self.connection_id = None;
        self.connected = false;

        Ok(())
    }

    async fn large_forward_open(&mut self) -> io::Result<()> {
        let cip = build_large_forward_open_request(self.slot, self.connection_params);

        let res = if self.connected {
            self.send_unit_data(cip).await?
        } else {
            self.send_rr_data(cip).await?
        };

        if res.len() < 10 {
            return Err(io::Error::other("LargeForwardOpen response too short"));
        }

        let status = res[2];
        if status != 0 {
            return Err(io::Error::other(format!(
                "LargeForwardOpen failed: 0x{:02X}",
                status
            )));
        }

        let conn_id = u32::from_le_bytes([res[6], res[7], res[8], res[9]]);
        self.connection_id = Some(conn_id);
        self.sequence = 1;
        self.connected = true;

        Ok(())
    }

    async fn large_forward_close(&mut self) -> io::Result<()> {
        if self.connection_id.is_none() {
            return Ok(());
        }

        let cip = build_large_forward_close_request(self.slot);

        if self.connected {
            let _ = self.send_unit_data(cip).await;
        } else {
            let _ = self.send_rr_data(cip).await;
        }

        self.connection_id = None;
        self.connected = false;

        Ok(())
    }

    async fn forward_open_with_fallback(&mut self) -> io::Result<()> {
        let res = {
            let cip = build_large_forward_open_request(self.slot, self.connection_params);
            if self.connected {
                self.send_unit_data(cip).await
            } else {
                self.send_rr_data(cip).await
            }
        };

        match res {
            Ok(_) => return Ok(()),

            Err(e) => {
                let msg = e.to_string();

                let should_fallback = msg.contains("0x01")
                    || msg.contains("0x20")
                    || msg.contains("0x26")
                    || msg.contains("0x05");

                if !should_fallback {
                    return Err(e);
                }
            }
        }

        self.forward_open()
            .await
            .map_err(|e| io::Error::other(e.to_string()))
    }

    async fn send_unit_data(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>> {
        let conn_id = self
            .connection_id
            .ok_or_else(|| io::Error::other("No active ForwardOpen connection"))?;

        let seq = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);

        let mut rr = Vec::new();
        rr.extend_from_slice(&conn_id.to_le_bytes());
        rr.extend_from_slice(&seq.to_le_bytes());
        rr.extend_from_slice(&cip);

        let pkt = EncapsulationHeader::new(COMMAND_SEND_UNIT_DATA, rr.len() as u16, self.session)
            .to_bytes()
            .to_vec();

        let mut full = pkt;
        full.extend_from_slice(&rr);

        self.stream.write_all(&full).await?;

        let mut h_buf = [0u8; 24];
        self.stream.read_exact(&mut h_buf).await?;
        let h = EncapsulationHeader::from_bytes(&h_buf)
            .ok_or_else(|| io::Error::other("Bad encapsulation header"))?;

        let mut d = vec![0u8; h.length as usize];
        self.stream.read_exact(&mut d).await?;

        Ok(super::EthernetIpClient::parse_cpf(&d)?.to_vec())
    }
}
