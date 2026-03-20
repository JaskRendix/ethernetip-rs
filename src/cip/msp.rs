use crate::cip::decode_cip_response;
use crate::types::{CipValue, MultiResult};

pub fn build_cip_multiple_service_request(requests: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();

    out.push(0x0A);
    out.push(0x00);

    out.extend_from_slice(&(requests.len() as u16).to_le_bytes());

    let mut offsets = Vec::new();
    let mut current_offset = 2 + 2 + (requests.len() * 2) as u16;

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

        if header_end >= buf.len() {
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

    if buf[0] != 0xCC || buf[1] != 0x00 {
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
