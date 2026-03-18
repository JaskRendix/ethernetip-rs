use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::encapsulation::{EncapsulationHeader, COMMAND_REGISTER_SESSION, COMMAND_SEND_RR_DATA};

pub async fn run_fake_plc() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:44818").await?;
    println!("Fake PLC listening on 127.0.0.1:44818");

    loop {
        let (mut socket, _) = listener.accept().await?;
        println!("Client connected");

        tokio::spawn(async move {
            let mut buf = [0u8; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(0) => return,
                    Ok(n) => n,
                    Err(_) => return,
                };

                if n < 24 {
                    continue;
                }

                let header = EncapsulationHeader::from_bytes(&buf[..24]);
                if header.is_none() {
                    continue;
                }
                let header = header.unwrap();

                match header.command {
                    COMMAND_REGISTER_SESSION => {
                        println!("Received RegisterSession");

                        let session = 0x12345678u32;

                        let mut resp =
                            EncapsulationHeader::new(COMMAND_REGISTER_SESSION, 4, session)
                                .to_bytes()
                                .to_vec();

                        resp.extend_from_slice(&session.to_le_bytes());

                        let _ = socket.write_all(&resp).await;
                    }

                    COMMAND_SEND_RR_DATA => {
                        println!("Received SendRRData");

                        // Fake CIP response: BOOL = true
                        let cip_response: Vec<u8> = vec![
                            0xCC, // service | 0x80 (0x4C + 0x80)
                            0x03, // path size in words (6 bytes)
                            0x91, 0x04, // ANSI symbol, length = 4
                            b'T', b'e', b's', b't',
                            // NO general_status, NO ext_status_size here
                            0xC1, // CIP type BOOL
                            0x00, // pad
                            0x01, // TRUE
                        ];

                        let mut rr = Vec::new();
                        rr.extend_from_slice(&0u32.to_le_bytes()); // interface handle
                        rr.extend_from_slice(&0u16.to_le_bytes()); // timeout
                        rr.extend_from_slice(&2u16.to_le_bytes()); // item count

                        rr.extend_from_slice(&0u16.to_le_bytes()); // null address type
                        rr.extend_from_slice(&0u16.to_le_bytes()); // null address length

                        rr.extend_from_slice(&0x00B2u16.to_le_bytes()); // unconnected data item
                        rr.extend_from_slice(&(cip_response.len() as u16).to_le_bytes());
                        rr.extend_from_slice(&cip_response);

                        let mut resp = EncapsulationHeader::new(
                            COMMAND_SEND_RR_DATA,
                            rr.len() as u16,
                            header.session,
                        )
                        .to_bytes()
                        .to_vec();

                        resp.extend_from_slice(&rr);

                        let _ = socket.write_all(&resp).await;
                    }

                    _ => {
                        println!("Unknown command: 0x{:04X}", header.command);
                    }
                }
            }
        });
    }
}
