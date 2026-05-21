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

        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;

        // Join CIP multicast group (239.192.1.1)
        let _ = socket.join_multicast_v4(
            std::net::Ipv4Addr::new(239, 192, 1, 1),
            std::net::Ipv4Addr::new(0, 0, 0, 0),
        );

        // Send ListIdentity request via broadcast
        let msg = EncapsulationHeader::new(COMMAND_LIST_IDENTITY, 0, 0).to_bytes();
        socket
            .send_to(&msg, SocketAddr::from(([255, 255, 255, 255], ENIP_PORT)))
            .await?;

        // Send ListIdentity request via multicast
        let _ = socket
            .send_to(&msg, SocketAddr::from(([239, 192, 1, 1], ENIP_PORT)))
            .await;

        discover_internal(socket).await
    }
}

impl DiscoveryImpl {
    /// Return the first discovered device, if any.
    pub async fn discover_one() -> io::Result<Option<DiscoveredIdentity>> {
        let all = Self::discover().await?;
        Ok(all.into_iter().next())
    }

    /// Test-only: run discovery using an injected socket.
    #[cfg(test)]
    pub async fn discover_with_socket(socket: UdpSocket) -> io::Result<Vec<DiscoveredIdentity>> {
        discover_internal(socket).await
    }

    #[cfg(test)]
    pub async fn discover_one_with_socket(
        socket: UdpSocket,
    ) -> io::Result<Option<DiscoveredIdentity>> {
        let all = discover_internal(socket).await?;
        Ok(all.into_iter().next())
    }
}

/// Shared discovery logic for production + tests.
async fn discover_internal(socket: UdpSocket) -> io::Result<Vec<DiscoveredIdentity>> {
    const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(1);
    const RECV_TIMEOUT: Duration = Duration::from_millis(200);

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
                let item_len = u16::from_le_bytes([payload[pos + 2], payload[pos + 3]]) as usize;
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

    results.dedup_by_key(|d| (d.ip.clone(), d.serial));

    Ok(results)
}

/// Parse the Identity Item (0x000C) returned by ListIdentity (0x63).
// Identity Item layout (EtherNet/IP spec Vol 2, Table 2-4.4):
//  [0..1]   encap protocol version
//  [2..9]   socket address (sin_family, sin_port, sin_addr, padding)
//  [10..11] vendor ID
//  [12..13] device type
//  [14..15] product code
//  [16]     revision major
//  [17]     revision minor
//  [18..19] status
//  [20..23] serial number
//  [24]     product name length
//  [25..]   product name (UTF-8)
/// This is *not* the same as the Identity Object (Class 0x01).
fn parse_discovery_identity(item: &[u8], ip: String) -> Option<DiscoveredIdentity> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UdpSocket;

    fn build_list_identity_packet(name: &str, serial: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&1u16.to_le_bytes());

        let mut identity = Vec::new();
        identity.extend_from_slice(&1u16.to_le_bytes());
        identity.extend_from_slice(&2u16.to_le_bytes());
        identity.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        identity.extend_from_slice(&123u16.to_le_bytes());
        identity.extend_from_slice(&14u16.to_le_bytes());
        identity.extend_from_slice(&999u16.to_le_bytes());
        identity.push(3);
        identity.push(7);
        identity.extend_from_slice(&0x0044u16.to_le_bytes());
        identity.extend_from_slice(&serial.to_le_bytes()); // <-- was hardcoded
        identity.push(name.len() as u8);
        identity.extend_from_slice(name.as_bytes());

        payload.extend_from_slice(&0x000Cu16.to_le_bytes());
        payload.extend_from_slice(&(identity.len() as u16).to_le_bytes());
        payload.extend_from_slice(&identity);

        let hdr =
            EncapsulationHeader::new(COMMAND_LIST_IDENTITY, payload.len() as u16, 0).to_bytes();
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&hdr);
        pkt.extend_from_slice(&payload);
        pkt
    }

    /// Returns (server, client).
    /// client is unconnected — recv_from will return the server's real addr.
    /// server is unconnected — must use send_to(src) to reply.
    async fn setup_socket_pair() -> (UdpSocket, SocketAddr, UdpSocket) {
        let server = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client_addr = client.local_addr().unwrap();
        (server, client_addr, client)
    }

    #[tokio::test]
    async fn test_single_device_discovery() {
        let (server, client_addr, client) = setup_socket_pair().await;
        tokio::spawn(async move {
            server
                .send_to(
                    &build_list_identity_packet("TestPLC", 0x11223344),
                    client_addr,
                )
                .await
                .unwrap();
        });
        let results = DiscoveryImpl::discover_with_socket(client).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].product_name, "TestPLC");
    }

    #[tokio::test]
    async fn test_multiple_devices() {
        let (server, client_addr, client) = setup_socket_pair().await;
        tokio::spawn(async move {
            server
                .send_to(
                    &build_list_identity_packet("PLC_A", 0x11223344),
                    client_addr,
                )
                .await
                .unwrap();
            server
                .send_to(
                    &build_list_identity_packet("PLC_B", 0x22334455),
                    client_addr,
                )
                .await
                .unwrap();
        });
        let results = DiscoveryImpl::discover_with_socket(client).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_ignores_wrong_command() {
        let (server, client_addr, client) = setup_socket_pair().await;
        tokio::spawn(async move {
            let mut pkt = build_list_identity_packet("Ignored", 0x11223344);
            pkt[0] = 0xFF;
            server.send_to(&pkt, client_addr).await.unwrap();
        });
        let results = DiscoveryImpl::discover_with_socket(client).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_ignores_malformed_identity_item() {
        let (server, client_addr, client) = setup_socket_pair().await;
        tokio::spawn(async move {
            server.send_to(&[1, 2, 3], client_addr).await.unwrap();
        });
        let results = DiscoveryImpl::discover_with_socket(client).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_discover_one() {
        let (server, client_addr, client) = setup_socket_pair().await;
        tokio::spawn(async move {
            server
                .send_to(
                    &build_list_identity_packet("OnePLC", 0x11223344),
                    client_addr,
                )
                .await
                .unwrap();
        });
        let result = DiscoveryImpl::discover_one_with_socket(client)
            .await
            .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().product_name, "OnePLC");
    }

    #[tokio::test]
    async fn test_discover_one_empty() {
        let (_server, _client_addr, client) = setup_socket_pair().await;
        let result = DiscoveryImpl::discover_one_with_socket(client)
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
