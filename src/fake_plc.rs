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
                        // interface handle + timeout
                        cpf.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                        // item count = 2
                        cpf.extend_from_slice(&2u16.to_le_bytes());
                        // null address item
                        cpf.extend_from_slice(&[0, 0, 0, 0]);
                        // data item (0x00B2)
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

                    COMMAND_SEND_UNIT_DATA => {
                        // treat UnitData exactly like RRData for fake PLC
                        if payload_len < 6 {
                            continue;
                        }

                        // first 4 bytes = connection ID
                        // next 2 bytes = sequence counter
                        // remaining = CIP
                        if payload_len < 6 {
                            continue;
                        }

                        let cip = &payload[6..];
                        if cip.is_empty() {
                            continue;
                        }

                        let cip_reply = handle_cip_request(cip, error_mode, &mut error_counter);

                        let mut cpf = Vec::new();
                        cpf.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // interface handle + timeout
                        cpf.extend_from_slice(&2u16.to_le_bytes());
                        cpf.extend_from_slice(&[0, 0, 0, 0]); // null address
                        cpf.extend_from_slice(&0x00B2u16.to_le_bytes());
                        cpf.extend_from_slice(&(cip_reply.len() as u16).to_le_bytes());
                        cpf.extend_from_slice(&cip_reply);

                        let hdr = EncapsulationHeader::new(
                            COMMAND_SEND_UNIT_DATA,
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
            // generic error every 5th request
            return vec![service | 0x80, 0x00, 0x01, 0x00];
        }
    }

    match service {
        0x52 => handle_read_fragmented(cip),
        0x54 => handle_forward_open(),
        0x4C => handle_read(cip),
        0x4D => handle_write(cip),
        0x4E => handle_forward_close(),
        0x03 if cip.len() >= 4 && cip[2] == 0x20 && cip[3] == 0x6B => handle_symbol_browse(),
        0x0A => handle_msp(cip, error_mode, error_counter),
        _ => vec![service | 0x80, 0x00, 0x01, 0x00],
    }
}

fn handle_forward_close() -> Vec<u8> {
    vec![
        0x4E | 0x80, // service | 0x80
        0x00,
        0x00, // success
        0x00,
    ]
}

fn handle_forward_open() -> Vec<u8> {
    // success reply with fake connection ID 0x11223344
    let mut out = vec![
        0x54 | 0x80, // service | 0x80
        0x00,        // reserved
        0x00,        // general status = success
        0x00,        // ext status size
    ];

    // fake O->T connection ID
    out.extend_from_slice(&0x11223344u32.to_le_bytes());

    // fake T->O connection ID
    out.extend_from_slice(&0x55667788u32.to_le_bytes());

    out
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

    let tag = extract_tag_name(cip);

    match tag.as_str() {
        "DINTTag" | "Test" | "Tag" => fake_read_dint(elem_count),
        "LINTTag" => fake_read_lint(elem_count),
        "REALTag" => fake_read_real(elem_count),
        "BOOLTag" => fake_read_bool(elem_count),
        "SINTTag" => fake_read_sint(elem_count),
        "INTTag" => fake_read_int(elem_count),
        "PackedBoolTag" => fake_read_bool_packed(elem_count),
        "StringTag" => fake_read_string(),
        _ => fake_read_dint(elem_count),
    }
}

fn handle_read_fragmented(cip: &[u8]) -> Vec<u8> {
    let path_words = cip[1] as usize;
    let path_bytes = path_words * 2;
    let elem_off = 2 + path_bytes;

    if cip.len() < elem_off + 6 {
        return vec![0xD2, 0x00, 0x01, 0x00];
    }

    let count = u16::from_le_bytes([cip[elem_off], cip[elem_off + 1]]) as usize;
    let offset = u32::from_le_bytes([
        cip[elem_off + 2],
        cip[elem_off + 3],
        cip[elem_off + 4],
        cip[elem_off + 5],
    ]) as usize;

    let tag = extract_tag_name(cip);

    let (type_id, full) = match tag.as_str() {
        "StringTag" => {
            let s = b"Hello";
            let mut v = Vec::new();
            v.extend_from_slice(&(s.len() as u16).to_le_bytes());
            v.extend_from_slice(s);
            v.extend(std::iter::repeat_n(0, 82 - s.len()));
            (0x00D0u16, v)
        }
        "LINTTag" => {
            let mut v = Vec::new();
            for i in 0..count {
                let val: i64 = 42000000000 + i as i64;
                v.extend_from_slice(&val.to_le_bytes());
            }
            (0x00C5u16, v)
        }
        "SINTTag" => {
            let v: Vec<u8> = (0..count).map(|i| (10 + i) as u8).collect();
            (0x00C2u16, v)
        }
        "INTTag" => {
            let mut v = Vec::new();
            for i in 0..count {
                let val: i16 = 1000 + i as i16;
                v.extend_from_slice(&val.to_le_bytes());
            }
            (0x00C3u16, v)
        }
        "PackedBoolTag" => (0x00D3u16, vec![0b01010101]),
        _ => {
            let mut v = Vec::new();
            for i in 0..count {
                let val: i32 = 42 + i as i32;
                v.extend_from_slice(&val.to_le_bytes());
            }
            (0x00C4u16, v)
        }
    };

    if offset >= full.len() && offset != 0 {
        return vec![0xD2, 0x00, 0x01, 0x00];
    }

    let remaining = &full[offset..];
    // determine if there is more data after this chunk (status 0x06) or we're done (0x00)
    let status: u8 = 0x00; // fake PLC always returns everything in one shot

    let mut out = vec![0x52 | 0x80, 0x00, status, 0x00];

    if offset == 0 {
        out.extend_from_slice(&type_id.to_le_bytes());
    }

    out.extend_from_slice(remaining);
    out
}

fn extract_tag_name(cip: &[u8]) -> String {
    if cip.len() < 4 {
        return "DINTTag".into();
    }

    let path_words = cip[1] as usize;
    let path_bytes = path_words * 2;

    if cip.len() < 2 + path_bytes {
        return "DINTTag".into();
    }

    let path = &cip[2..2 + path_bytes];

    if path.len() < 2 || path[0] != 0x91 {
        return "DINTTag".into();
    }

    let len = path[1] as usize;
    if path.len() < 2 + len {
        return "DINTTag".into();
    }

    let name_bytes = &path[2..2 + len];
    String::from_utf8_lossy(name_bytes).into_owned()
}

fn fake_read_dint(count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xC4, 0x00];
    for i in 0..count {
        let v: i32 = 42 + i as i32;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn fake_read_lint(count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xC5, 0x00];
    for i in 0..count {
        let v: i64 = 42000000000 + i as i64;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn fake_read_real(count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xCA, 0x00];
    for i in 0..count {
        let v: f32 = 1.5 + i as f32;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn fake_read_bool(count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xC1, 0x00];
    for i in 0..count {
        out.push(if i % 2 == 0 { 1 } else { 0 });
    }
    out
}

fn fake_read_bool_packed(_count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xD3, 0x00];
    out.push(0b01010101);
    out
}

fn fake_read_string() -> Vec<u8> {
    let s = b"Hello";
    let len = s.len() as u16;

    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xD0, 0x00];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(s);
    out.extend(std::iter::repeat_n(0, 82 - s.len()));
    out
}

fn fake_read_sint(count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xC2, 0x00];
    for i in 0..count {
        out.push((10 + i as i8) as u8);
    }
    out
}

fn fake_read_int(count: usize) -> Vec<u8> {
    let mut out = vec![0x4C | 0x80, 0x00, 0x00, 0x00, 0xC3, 0x00];
    for i in 0..count {
        let v: i16 = 1000 + i as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn handle_write(cip: &[u8]) -> Vec<u8> {
    if cip.len() < 6 {
        return vec![0xCD, 0x00, 0x01, 0x00];
    }

    let path_words = cip[1] as usize;
    let path_bytes = path_words * 2;
    let mut pos = 2 + path_bytes;

    if cip.len() < pos + 4 {
        return vec![0xCD, 0x00, 0x01, 0x00];
    }

    let typ = u16::from_le_bytes([cip[pos], cip[pos + 1]]);
    pos += 2;

    let elem_count = u16::from_le_bytes([cip[pos], cip[pos + 1]]) as usize;
    pos += 2;

    let ok = match typ {
        0x00C1 => cip.len() >= pos + elem_count,
        0x00C2 => cip.len() >= pos + elem_count,
        0x00C3 => cip.len() >= pos + elem_count * 2,
        0x00C4 => cip.len() >= pos + elem_count * 4,
        0x00C5 => cip.len() >= pos + elem_count * 8,
        0x00CA => cip.len() >= pos + elem_count * 4,
        0x00D0 => cip.len() >= pos + 2,
        0x00D3 => cip.len() > pos,
        _ => false,
    };

    if !ok {
        return vec![0xCD, 0x00, 0x01, 0x00];
    }

    vec![0xCD, 0x00, 0x00, 0x00]
}

fn handle_symbol_browse() -> Vec<u8> {
    let name = b"TestTag";
    let name_len = name.len() as u8;

    let mut block = Vec::new();

    // service 0x83, success
    block.extend_from_slice(&[0x83, 0x00, 0x00, 0x00]);

    // symbol handle
    block.extend_from_slice(&0u16.to_le_bytes());
    block.push(name_len);
    block.extend_from_slice(name);
    if !name_len.is_multiple_of(2) {
        block.push(0);
    }

    // symbol type = DINT
    block.extend_from_slice(&0u16.to_le_bytes());
    block.extend_from_slice(&0xC4u16.to_le_bytes());

    // array dims = none
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
