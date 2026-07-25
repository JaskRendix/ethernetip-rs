use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::cip::{
    build_forward_close_request, build_forward_open_request, build_large_forward_close_request,
    build_large_forward_open_request, decode_extended_status, describe_extended_status,
    ConnectionIds, ForwardOpenError,
};
use crate::client::UnconnectedMessaging;
use crate::encapsulation::*;

use super::{ConnectedMessaging, EthernetIpClient};

#[async_trait::async_trait]
impl ConnectedMessaging for EthernetIpClient {
    async fn forward_open(&mut self) -> Result<(), ForwardOpenError> {
        let ids = ConnectionIds::default();
        let cip = build_forward_open_request(self.slot, self.connection_params, ids);

        let res = if self.connected {
            self.send_unit_data(cip).await?
        } else {
            self.send_rr_data(cip).await?
        };

        if res.len() < 4 {
            return Err(ForwardOpenError::Other(
                "ForwardOpen response too short".into(),
            ));
        }

        let general = res[2];
        let ext_words = res[3] as usize;
        let ext_bytes = 4 + ext_words * 2;

        if res.len() < ext_bytes + 4 {
            return Err(ForwardOpenError::Other(
                "ForwardOpen response missing connection ID".into(),
            ));
        }

        if general != 0 {
            let ext = decode_extended_status(&res);

            if !ext.is_empty() {
                if let Some(mapped) = describe_extended_status(&ext) {
                    return Err(ForwardOpenError::Other(mapped));
                }
            }
            return Err(ForwardOpenError::GeneralStatus(general));
        }

        let conn_id = u32::from_le_bytes([
            res[ext_bytes],
            res[ext_bytes + 1],
            res[ext_bytes + 2],
            res[ext_bytes + 3],
        ]);

        self.connection_id = Some(conn_id);
        self.sequence = 1;
        self.connected = true;

        Ok(())
    }

    async fn forward_close(&mut self) -> io::Result<()> {
        if self.connection_id.is_none() {
            return Ok(());
        }

        let ids = ConnectionIds::default();
        let cip = build_forward_close_request(self.slot, ids);

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
        let ids = ConnectionIds::default();
        let cip = build_large_forward_open_request(self.slot, self.connection_params, ids);

        let res = if self.connected {
            self.send_unit_data(cip).await?
        } else {
            self.send_rr_data(cip).await?
        };

        if res.len() < 4 {
            return Err(io::Error::other("LargeForwardOpen response too short"));
        }

        let general = res[2];
        let ext_words = res[3] as usize;
        let ext_bytes = 4 + ext_words * 2;

        if res.len() < ext_bytes + 4 {
            return Err(io::Error::other(
                "LargeForwardOpen response missing connection ID",
            ));
        }

        if general != 0 {
            return Err(io::Error::other(format!(
                "LargeForwardOpen failed: 0x{:02X}",
                general
            )));
        }

        let conn_id = u32::from_le_bytes([
            res[ext_bytes],
            res[ext_bytes + 1],
            res[ext_bytes + 2],
            res[ext_bytes + 3],
        ]);

        self.connection_id = Some(conn_id);
        self.sequence = 1;
        self.connected = true;

        Ok(())
    }

    async fn large_forward_close(&mut self) -> io::Result<()> {
        if self.connection_id.is_none() {
            return Ok(());
        }

        let ids = ConnectionIds::default();
        let cip = build_large_forward_close_request(self.slot, ids);

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
        // Try Large Forward Open first. large_forward_open() already parses
        // the response and sets connection_id/sequence/connected on success,
        // so delegate to it rather than re-sending the request by hand here
        // (a prior version of this function sent the request directly and
        // discarded the response, leaving connection state unset on success).
        match self.large_forward_open().await {
            Ok(()) => return Ok(()),
            Err(e) => {
                let msg = e.to_string();

                // TODO: this matches on formatted error text from
                // large_forward_open()'s "LargeForwardOpen failed: 0x{:02X}"
                // string. It works today but is fragile if that format
                // string ever changes. Prefer a typed status code once
                // large_forward_open() can return one instead of io::Error.
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
        self.sequence = if self.sequence == u16::MAX {
            1
        } else {
            self.sequence.wrapping_add(1)
        };

        let mut rr = Vec::with_capacity(4 + 2 + cip.len());
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

        if h.command != COMMAND_SEND_UNIT_DATA {
            return Err(io::Error::other(
                "Unexpected encapsulation command in response",
            ));
        }

        let mut d = vec![0u8; h.length as usize];
        self.stream.read_exact(&mut d).await?;

        Ok(super::EthernetIpClient::parse_cpf(&d)?.to_vec())
    }
}
