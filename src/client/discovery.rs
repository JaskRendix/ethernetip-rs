use std::io;
use std::net::SocketAddr;
use std::time::Instant;

use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};

use crate::encapsulation::*;

use super::Discovery;

pub struct DiscoveryImpl;

#[async_trait::async_trait]
impl Discovery for DiscoveryImpl {
    async fn discover() -> io::Result<Vec<(String, String)>> {
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
}
