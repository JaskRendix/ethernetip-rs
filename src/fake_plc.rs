use crate::encapsulation::*;
use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub async fn run_fake_plc() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:44818").await?;
    println!("Fake PLC listening on 127.0.0.1:44818");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut session_id = 0u32;
            let mut header = [0u8; EncapsulationHeader::SIZE];
            let mut payload = Vec::new();

            let error_mode = env::var("FAKE_PLC_ERROR").is_ok();
            let mut error_counter: u32 = 0;

            loop {
                if socket.read_exact(&mut header).await.is_err() {
                    break;
                }

                let hdr = match EncapsulationHeader::from_bytes(&header) {
                    Some(h) => h,
                    None => break,
                };

                let payload_len = hdr.length as usize;
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

                        let item2_len = u16::from_le_bytes([payload[14], payload[15]]) as usize;
                        if payload_len < 16 + item2_len {
                            continue;
                        }

                        let cip = &payload[16..16 + item2_len];
                        if cip.is_empty() {
                            continue;
                        }

                        let cip_reply = handle_cip_request(cip, error_mode, &mut error_counter);

                        let mut cpf = Vec::new();
                        cpf.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // interface, timeout
                        cpf.extend_from_slice(&2u16.to_le_bytes()); // item count
                        cpf.extend_from_slice(&[0, 0, 0, 0]); // null addr item
                        cpf.extend_from_slice(&0x00B2u16.to_le_bytes()); // unconnected data
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

fn handle_cip_request(cip: &[u8], error_mode: bool, error_counter: &mut u32) -> Vec<u8> {
    let service = cip[0];

    if error_mode {
        *error_counter += 1;
        if (*error_counter).is_multiple_of(5) {
            return vec![service | 0x80, 0x00, 0x01, 0x00];
        }
    }

    match service {
        0x4C => handle_read(cip),
        0x4D => handle_write(),
        0x03 if cip.len() >= 4 && cip[2] == 0x20 && cip[3] == 0x6B => handle_symbol_browse(),
        0x0A => handle_msp(cip, error_mode, error_counter),
        _ => vec![service | 0x80, 0x00, 0x01, 0x00],
    }
}

fn handle_read(cip: &[u8]) -> Vec<u8> {
    if cip.len() < 3 {
        return vec![0xCC, 0x00, 0x01, 0x00];
    }

    let path_words = cip[1] as usize;
    let path_bytes = path_words * 2;
    let elem_off = 2 + path_bytes;

    if cip.len() < elem_off + 2 {
        return vec![0xCC, 0x00, 0x01, 0x00];
    }

    let elem_count = u16::from_le_bytes([cip[elem_off], cip[elem_off + 1]]) as usize;
    let elem_count = elem_count.max(1);

    let mut out = Vec::new();
    out.extend_from_slice(&[0xCC, 0x00, 0x00, 0x00]); // reply header
    out.extend_from_slice(&[0xC4, 0x00]); // DINT

    for i in 0..elem_count {
        let v: i32 = 42 + i as i32;
        out.extend_from_slice(&v.to_le_bytes());
    }

    out
}

fn handle_write() -> Vec<u8> {
    vec![0xCC, 0x00, 0x00, 0x00]
}

fn handle_symbol_browse() -> Vec<u8> {
    let name = b"TestTag";
    let name_len = name.len() as u8;

    let mut block = Vec::new();

    block.extend_from_slice(&[0x83, 0x00, 0x00, 0x00]);

    block.extend_from_slice(&0u16.to_le_bytes());
    block.push(name_len);
    block.extend_from_slice(name);
    if !name_len.is_multiple_of(2) {
        block.push(0);
    }

    block.extend_from_slice(&0u16.to_le_bytes());
    block.extend_from_slice(&0xC4u16.to_le_bytes());

    block.extend_from_slice(&0u16.to_le_bytes());
    block.push(0);
    block.extend_from_slice(&0u16.to_le_bytes());
    block.extend_from_slice(&0u16.to_le_bytes());
    block.extend_from_slice(&0u16.to_le_bytes());

    block.extend_from_slice(&0u16.to_le_bytes());

    block
}

fn handle_msp(cip: &[u8], error_mode: bool, error_counter: &mut u32) -> Vec<u8> {
    if cip.len() < 4 {
        return vec![0x8A, 0x00, 0x00, 0x00];
    }

    let count = u16::from_le_bytes([cip[2], cip[3]]) as usize;
    if count == 0 || count > 64 {
        return vec![0x8A, 0x00, 0x00, 0x00];
    }

    let offsets_start = 4;
    let offsets_end = offsets_start + count * 2;
    if cip.len() < offsets_end {
        return vec![0x8A, 0x00, 0x00, 0x00];
    }

    let mut offsets = Vec::with_capacity(count);
    for i in 0..count {
        let o = offsets_start + i * 2;
        offsets.push(u16::from_le_bytes([cip[o], cip[o + 1]]) as usize);
    }

    for w in offsets.windows(2) {
        if w[1] < w[0] {
            return vec![0x8A, 0x00, 0x00, 0x00];
        }
    }

    let mut replies = Vec::new();
    let mut reply_offsets = Vec::with_capacity(count);

    let base_offset = 2 + 2 + count * 2;
    let mut current_offset = base_offset as u16;

    for off in offsets {
        if off >= cip.len() {
            replies.push(vec![0xCC, 0x00, 0x01, 0x00]);
        } else {
            let req = &cip[off..];
            let r = handle_cip_request(req, error_mode, error_counter);
            replies.push(r);
        }
    }

    for r in &replies {
        reply_offsets.push(current_offset);
        current_offset += r.len() as u16;
    }

    let mut out = Vec::new();
    out.push(0x8A);
    out.push(0x00);
    out.extend_from_slice(&(count as u16).to_le_bytes());

    for off in &reply_offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }

    for r in replies {
        out.extend_from_slice(&r);
    }

    out
}
