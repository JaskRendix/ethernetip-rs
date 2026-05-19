use std::io;

use crate::cip::CipError;
use crate::client::build_read_fragmented_request;
use crate::client::{ConnectedMessaging, UnconnectedMessaging};
use crate::types::CipValue;

use super::{EthernetIpClient, FragmentedRead};

#[async_trait::async_trait]
impl FragmentedRead for EthernetIpClient {
    async fn read_tag_fragmented(
        &mut self,
        tag: &str,
        count: u16,
    ) -> Result<(u16, Vec<u8>), CipError> {
        let mut all_data = Vec::new();
        let mut offset: u32 = 0;
        let mut type_id: u16 = 0;

        loop {
            let cip = build_read_fragmented_request(tag, count, offset, self.slot);

            // Explicit type annotation fixes E0282
            let res: io::Result<Vec<u8>> = if self.connected {
                self.send_unit_data(cip).await
            } else {
                self.send_rr_data(cip).await
            };

            let res = res.map_err(|_| CipError::VendorSpecific(0xFF))?;

            if res.len() < 4 {
                return Err(CipError::VendorSpecific(0xFE));
            }

            let general_status = res[2];
            let ext_words = res[3] as usize;
            let data_start = 4 + (ext_words * 2);

            if res.len() < data_start {
                return Err(CipError::VendorSpecific(0xFD));
            }

            let mut payload = &res[data_start..];

            if offset == 0 {
                if payload.len() < 2 {
                    return Err(CipError::VendorSpecific(0xFC));
                }
                type_id = u16::from_le_bytes([payload[0], payload[1]]);
                payload = &payload[2..];
            }

            all_data.extend_from_slice(payload);

            match general_status {
                0x00 => break,
                0x06 => offset = all_data.len() as u32,
                other => return Err(CipError::from(other)),
            }
        }

        Ok((type_id, all_data))
    }

    async fn read_array(&mut self, tag: &str, count: u16) -> Result<Vec<CipValue>, CipError> {
        let (type_id, raw) = self.read_tag_fragmented(tag, count).await?;
        Ok(crate::cip::decode_cip_data_list(type_id, &raw))
    }
}
