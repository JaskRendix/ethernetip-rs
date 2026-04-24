use crate::cip::decode_cip_response;
use crate::types::{CipValue, MultiResult};

pub fn build_cip_multiple_service_request(requests: &[Vec<u8>]) -> Vec<u8> {
    let count = requests.len();
    let header_len = 2 + 2 + count * 2;

    let total_payload: usize = requests.iter().map(|r| r.len()).sum();
    let mut out = Vec::with_capacity(header_len + total_payload);

    out.push(0x0A);
    out.push(0x00);

    out.extend_from_slice(&(count as u16).to_le_bytes());

    let mut offsets = Vec::with_capacity(count);
    let mut current_offset = header_len as u16;

    for req in requests {
        offsets.push(current_offset);
        current_offset += req.len() as u16;
    }

    for off in offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }

    for req in requests {
        out.extend_from_slice(req);
    }

    out
}

pub fn parse_cip_multiple_service_response(buf: &[u8]) -> Vec<MultiResult<CipValue>> {
    let mut results = Vec::new();

    if buf.len() < 4 {
        return results;
    }

    let count = u16::from_le_bytes([buf[2], buf[3]]) as usize;

    if count == 0 || count > 64 {
        return results;
    }

    let offsets_start = 4;
    let offsets_end = offsets_start + count * 2;
    if buf.len() < offsets_end {
        return results;
    }

    let mut offsets = Vec::with_capacity(count);
    for i in 0..count {
        let o = offsets_start + i * 2;
        offsets.push(u16::from_le_bytes([buf[o], buf[o + 1]]) as usize);
    }

    for w in offsets.windows(2) {
        if w[1] < w[0] {
            results.resize(count, MultiResult::Err(0xFF));
            return results;
        }
    }
    for off in &offsets {
        if *off < offsets_end {
            results.push(MultiResult::Err(0xFF));
            continue;
        }
    }

    for off in offsets {
        if off + 4 > buf.len() {
            results.push(MultiResult::Err(0xFF));
            continue;
        }

        if buf[off] != 0xCC || buf[off + 1] != 0x00 {
            results.push(MultiResult::Err(0xFF));
            continue;
        }

        let general_status = buf[off + 2];
        let ext_words = buf[off + 3] as usize;
        let ext_bytes = ext_words * 2;

        let header_end = off + 4 + ext_bytes;
        if header_end > buf.len() {
            results.push(MultiResult::Err(0xFF));
            continue;
        }

        if general_status != 0 {
            results.push(MultiResult::Err(general_status));
            continue;
        }

        if header_end == buf.len() {
            results.push(MultiResult::Ok(CipValue::Unit));
            continue;
        }

        let cip_buf = &buf[header_end..];

        match decode_cip_response(cip_buf) {
            Some(v) => results.push(MultiResult::Ok(v)),
            None => results.push(MultiResult::Err(0xFF)),
        }
    }

    results
}

pub fn decode_write_response(buf: &[u8]) -> Result<(), u8> {
    if buf.len() < 4 {
        return Err(0xFF);
    }
    if buf[0] != 0xCD || buf[1] != 0x00 {
        // 0xCD = 0x4D | 0x80 (write reply)
        return Err(0xFF);
    }
    let general_status = buf[2];
    let ext_words = buf[3] as usize;
    let needed = 4 + ext_words * 2;
    if buf.len() < needed {
        return Err(0xFF);
    }
    if general_status != 0 {
        return Err(general_status);
    }
    Ok(())
}
