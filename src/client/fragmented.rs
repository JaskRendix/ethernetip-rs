use std::io;

use crate::cip::{decode_extended_status, CipError};
use crate::client::build_read_fragmented_request;
use crate::client::{ConnectedMessaging, UnconnectedMessaging};
use crate::types::CipValue;

use super::{EthernetIpClient, FragmentedRead};

/// Safety cap on total fragments to avoid an unbounded loop against a
/// misbehaving or malicious device that keeps returning "more data" status
/// without making progress or without ever completing.
const MAX_FRAGMENTS: usize = 4096;

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
        let mut fragments: usize = 0;

        loop {
            fragments += 1;
            if fragments > MAX_FRAGMENTS {
                return Err(CipError::General {
                    status: 0xFF,
                    extended: vec![],
                });
            }

            let cip = build_read_fragmented_request(tag, count, offset, self.slot);

            let res: io::Result<Vec<u8>> = if self.connected {
                self.send_unit_data(cip).await
            } else {
                self.send_rr_data(cip).await
            };

            let res = res.map_err(CipError::from)?; // preserves the real IO error now

            if res.len() < 4 {
                return Err(CipError::General {
                    status: 0xFE,
                    extended: vec![],
                });
            }

            let general_status = res[2];
            let ext_words = res[3] as usize;
            let data_start = 4 + (ext_words * 2);

            if res.len() < data_start {
                return Err(CipError::General {
                    status: 0xFD,
                    extended: vec![],
                });
            }

            let mut payload = &res[data_start..];

            if offset == 0 {
                if payload.len() < 2 {
                    return Err(CipError::General {
                        status: 0xFC,
                        extended: vec![],
                    });
                }
                type_id = u16::from_le_bytes([payload[0], payload[1]]);
                payload = &payload[2..];
            }

            all_data.extend_from_slice(payload);

            match general_status {
                0x00 => break,
                0x06 => {
                    let new_offset = all_data.len() as u32;
                    if new_offset == offset {
                        return Err(CipError::General {
                            status: 0x06,
                            extended: vec![],
                        });
                    }
                    offset = new_offset;
                }
                other => {
                    let extended = decode_extended_status(&res[..data_start]);
                    return Err(CipError::General {
                        status: other,
                        extended,
                    });
                }
            }
        }

        Ok((type_id, all_data))
    }

    async fn read_array(&mut self, tag: &str, count: u16) -> Result<Vec<CipValue>, CipError> {
        let (type_id, raw) = self.read_tag_fragmented(tag, count).await?;
        Ok(crate::cip::decode_cip_data_list(type_id, &raw))
    }
}
