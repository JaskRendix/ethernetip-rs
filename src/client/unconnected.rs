use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

use crate::encapsulation::*;

use super::{EthernetIpClient, UnconnectedMessaging};

#[async_trait::async_trait]
impl UnconnectedMessaging for EthernetIpClient {
    async fn send_rr_data(&mut self, cip: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut attempt = 0;

        loop {
            let result = self.send_rr_data_inner(&cip).await;

            match result {
                Ok(data) => return Ok(data),

                Err(e)
                    if attempt < self.retries
                        && matches!(
                            e.kind(),
                            io::ErrorKind::ConnectionReset
                                | io::ErrorKind::BrokenPipe
                                | io::ErrorKind::UnexpectedEof
                        ) =>
                {
                    attempt += 1;
                    let backoff = Duration::from_millis(50 * attempt as u64);

                    tokio::time::sleep(backoff).await;
                    self.reconnect().await?;
                    continue;
                }

                Err(e) => return Err(e),
            }
        }
    }
}

impl EthernetIpClient {
    async fn send_rr_data_inner(&mut self, cip: &[u8]) -> io::Result<Vec<u8>> {
        let mut rr = Vec::with_capacity(22 + cip.len());
        rr.extend_from_slice(&0u32.to_le_bytes());
        rr.extend_from_slice(&0u16.to_le_bytes());
        rr.extend_from_slice(&2u16.to_le_bytes());

        rr.extend_from_slice(&0x0000u16.to_le_bytes());
        rr.extend_from_slice(&0u16.to_le_bytes());

        rr.extend_from_slice(&0x00B2u16.to_le_bytes());
        rr.extend_from_slice(&(cip.len() as u16).to_le_bytes());
        rr.extend_from_slice(cip);

        let mut pkt = EncapsulationHeader::new(COMMAND_SEND_RR_DATA, rr.len() as u16, self.session)
            .to_bytes()
            .to_vec();

        pkt.extend_from_slice(&rr);

        timeout(Duration::from_secs(3), async {
            self.stream.write_all(&pkt).await?;

            let mut h_buf = [0u8; 24];
            self.stream.read_exact(&mut h_buf).await?;
            let h = EncapsulationHeader::from_bytes(&h_buf)
                .ok_or_else(|| io::Error::other("Bad encapsulation header"))?;

            if h.length == 0 {
                return Err(io::Error::other("Empty encapsulation payload"));
            }

            let mut d = vec![0u8; h.length as usize];
            self.stream.read_exact(&mut d).await?;

            Ok(super::EthernetIpClient::parse_cpf(&d)?.to_vec())
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "PLC Timeout"))?
    }
}
