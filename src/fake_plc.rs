use crate::encapsulation::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run_fake_plc() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:44818").await?;
    println!("🤖 Fake PLC listening on 127.0.0.1:44818");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut session_id = 0u32;

            // Header is always 24 bytes
            let mut header = [0u8; 24];
            // Payload is dynamic
            let mut payload = Vec::new();

            loop {
                // --- READ HEADER (24 bytes) ---
                if socket.read_exact(&mut header).await.is_err() {
                    break;
                }

                let hdr = match EncapsulationHeader::from_bytes(&header) {
                    Some(h) => h,
                    None => break,
                };

                let payload_len = hdr.length as usize;

                // --- READ PAYLOAD (dynamic length) ---
                payload.resize(payload_len, 0);
                if socket.read_exact(&mut payload).await.is_err() {
                    break;
                }

                match hdr.command {
                    COMMAND_REGISTER_SESSION => {
                        session_id = 0x1234_5678;

                        let mut res =
                            EncapsulationHeader::new(COMMAND_REGISTER_SESSION, 4, session_id)
                                .to_bytes()
                                .to_vec();

                        res.extend_from_slice(&1u16.to_le_bytes());
                        res.extend_from_slice(&0u16.to_le_bytes());

                        let _ = socket.write_all(&res).await;
                    }

                    COMMAND_SEND_RR_DATA => {
                        if payload_len < 16 {
                            continue;
                        }

                        // item2_len = payload[14..16]
                        let item2_len = u16::from_le_bytes([payload[14], payload[15]]) as usize;

                        if payload_len < 16 + item2_len {
                            continue;
                        }

                        // CIP payload starts at payload[16]
                        let cip = &payload[16..16 + item2_len];

                        let service = cip[0];
                        let mut cip_reply = Vec::new();

                        if service == 0x4C {
                            // READ
                            cip_reply.extend_from_slice(&[0xCC, 0x00, 0x00, 0x00]);
                            cip_reply.extend_from_slice(&[0xC4, 0x00]);
                            cip_reply.extend_from_slice(&42i32.to_le_bytes());
                        } else if service == 0x4D {
                            // WRITE (strict Logix-family)
                            cip_reply.extend_from_slice(&[0xCC, 0x00, 0x00, 0x00]);
                        } else if service == 0x03
                            && cip.len() >= 4
                            && cip[2] == 0x20
                            && cip[3] == 0x6B
                        {
                            // Get_Attribute_List on Class 0x6B (Symbol Object)

                            let name = b"TestTag";
                            let name_len = name.len() as u8;

                            let mut block = Vec::new();

                            // CIP reply header: service|0x80, reserved, general status, ext words
                            block.extend_from_slice(&[0x03 | 0x80, 0x00, 0x00, 0x00]);

                            // Attr 1 status OK
                            block.extend_from_slice(&0u16.to_le_bytes());
                            block.push(name_len);
                            block.extend_from_slice(name);

                            // CIP‑spec padding: pad if name_len is odd
                            if !name_len.is_multiple_of(2) {
                                block.push(0);
                            }

                            // Attr 2 status OK
                            block.extend_from_slice(&0u16.to_le_bytes());
                            block.extend_from_slice(&(0xC4u16).to_le_bytes()); // DINT

                            // Attr 5 status OK
                            block.extend_from_slice(&0u16.to_le_bytes());
                            block.push(0); // dim count = 0

                            // Always send 3 dims (all zero)
                            block.extend_from_slice(&0u16.to_le_bytes());
                            block.extend_from_slice(&0u16.to_le_bytes());
                            block.extend_from_slice(&0u16.to_le_bytes());

                            // 2‑byte padding after symbol
                            block.extend_from_slice(&0u16.to_le_bytes());

                            // Wrap in CPF
                            let mut cpf = Vec::new();
                            cpf.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                            cpf.extend_from_slice(&2u16.to_le_bytes());
                            cpf.extend_from_slice(&[0, 0, 0, 0]);
                            cpf.extend_from_slice(&0x00B2u16.to_le_bytes());
                            cpf.extend_from_slice(&(block.len() as u16).to_le_bytes());
                            cpf.extend_from_slice(&block);

                            let hdr = EncapsulationHeader::new(
                                COMMAND_SEND_RR_DATA,
                                cpf.len() as u16,
                                session_id,
                            )
                            .to_bytes();

                            let _ = socket.write_all(&hdr).await;
                            let _ = socket.write_all(&cpf).await;
                            continue;
                        } else {
                            cip_reply.extend_from_slice(&[service | 0x80, 0x00, 0x01, 0x00]);
                        }

                        // Build CPF reply
                        let mut cpf = Vec::new();
                        cpf.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                        cpf.extend_from_slice(&2u16.to_le_bytes());
                        cpf.extend_from_slice(&[0, 0, 0, 0]);
                        cpf.extend_from_slice(&0x00B2u16.to_le_bytes());
                        cpf.extend_from_slice(&(cip_reply.len() as u16).to_le_bytes());
                        cpf.extend_from_slice(&cip_reply);

                        let hdr = EncapsulationHeader::new(
                            COMMAND_SEND_RR_DATA,
                            cpf.len() as u16,
                            session_id,
                        )
                        .to_bytes();

                        let _ = socket.write_all(&hdr).await;
                        let _ = socket.write_all(&cpf).await;
                    }

                    COMMAND_UNREGISTER_SESSION => break,

                    _ => {}
                }
            }
        });
    }
}
