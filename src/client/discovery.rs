use std::io;
use std::net::SocketAddr;
use std::time::Instant;

use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

use crate::encapsulation::*;
use crate::types::DiscoveredIdentity;

use super::Discovery;

pub struct DiscoveryImpl;

#[async_trait::async_trait]
impl Discovery for DiscoveryImpl {
    async fn discover() -> io::Result<Vec<DiscoveredIdentity>> {
        const ENIP_PORT: u16 = 44818;
        const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(1);
        const RECV_TIMEOUT: Duration = Duration::from_millis(200);

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;

        // Send ListIdentity request
        let msg = EncapsulationHeader::new(COMMAND_LIST_IDENTITY, 0, 0).to_bytes();
        socket
            .send_to(&msg, SocketAddr::from(([255, 255, 255, 255], ENIP_PORT)))
            .await?;

        let mut results = Vec::new();
        let mut buf = [0u8; 2048];
        let start = Instant::now();

        while start.elapsed() < DISCOVERY_TIMEOUT {
            if let Ok(Ok((len, addr))) = timeout(RECV_TIMEOUT, socket.recv_from(&mut buf)).await {
                if len < 24 {
                    continue;
                }

                // Parse encapsulation header
                let hdr = match EncapsulationHeader::from_bytes(&buf[..24]) {
                    Some(h) => h,
                    None => continue,
                };

                if hdr.command != COMMAND_LIST_IDENTITY || hdr.status != 0 {
                    continue;
                }

                let payload = &buf[24..len];
                if payload.len() < 2 {
                    continue;
                }

                // CPF item count
                let item_count = u16::from_le_bytes([payload[0], payload[1]]) as usize;
                let mut pos = 2;

                for _ in 0..item_count {
                    if payload.len() < pos + 4 {
                        break;
                    }

                    let type_id = u16::from_le_bytes([payload[pos], payload[pos + 1]]);
                    let item_len =
                        u16::from_le_bytes([payload[pos + 2], payload[pos + 3]]) as usize;
                    pos += 4;

                    if payload.len() < pos + item_len {
                        break;
                    }

                    // Identity Item = 0x000C
                    if type_id == 0x000C {
                        if let Some(info) = parse_discovery_identity(
                            &payload[pos..pos + item_len],
                            addr.ip().to_string(),
                        ) {
                            results.push(info);
                        }
                    }

                    pos += item_len;
                }
            }
        }

        Ok(results)
    }
}

/// Parse the Identity Item (0x000C) returned by ListIdentity (0x63).
///
/// This is *not* the same as the Identity Object (Class 0x01).
fn parse_discovery_identity(item: &[u8], ip: String) -> Option<DiscoveredIdentity> {
    // Minimum fields before product name:
    //  0-1:  Encapsulation Protocol Version
    //  2-3:  Socket Address Family
    //  4-9:  Socket Address (ignored)
    // 10-11: Vendor ID
    // 12-13: Device Type
    // 14-15: Product Code
    // 16:    Revision Major
    // 17:    Revision Minor
    // 18-19: Status
    // 20-23: Serial Number
    // 24:    Product Name Length
    if item.len() < 25 {
        return None;
    }

    let vendor_id = u16::from_le_bytes([item[10], item[11]]);
    let device_type = u16::from_le_bytes([item[12], item[13]]);
    let product_code = u16::from_le_bytes([item[14], item[15]]);
    let revision_major = item[16];
    let revision_minor = item[17];
    let status = u16::from_le_bytes([item[18], item[19]]);
    let serial = u32::from_le_bytes([item[20], item[21], item[22], item[23]]);

    let name_len = item[24] as usize;
    if item.len() < 25 + name_len {
        return None;
    }

    let product_name = String::from_utf8_lossy(&item[25..25 + name_len]).into_owned();

    Some(DiscoveredIdentity {
        ip,
        vendor_id,
        device_type,
        product_code,
        revision_major,
        revision_minor,
        status,
        serial,
        product_name,
    })
}
