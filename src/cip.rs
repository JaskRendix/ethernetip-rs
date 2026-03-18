pub use crate::types::CipValue;
use crate::types::{CipType, MultiResult};

#[derive(Debug, Clone, Copy)]
pub enum CipService {
    ReadData = 0x4C,
    WriteData = 0x4D,
    ReadFragment = 0x52,
    WriteFragment = 0x53,
    MultipleService = 0x0A,
}

impl CipService {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipError {
    ConnectionFailure,
    ResourceUnavailable,
    InvalidAttribute,
    PathSegmentError,
    PathDestinationUnknown,
    VendorSpecific(u8),
}

impl From<u8> for CipError {
    fn from(code: u8) -> Self {
        match code {
            0x01 => CipError::ConnectionFailure,
            0x02 => CipError::ResourceUnavailable,
            0x04 => CipError::InvalidAttribute,
            0x05 => CipError::PathSegmentError,
            0x06 => CipError::PathDestinationUnknown,
            other => CipError::VendorSpecific(other),
        }
    }
}

/// Encode a tag path like "Tag", "Struct.Member", "Array[3]", "Array[1,2]"
pub fn encode_epath(tag: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut segments = Vec::new();

    for part in tag.split('.') {
        let mut name = part;
        let mut indices: Vec<u32> = Vec::new();

        if let Some(idx_start) = part.find('[') {
            name = &part[..idx_start];
            let idx_str = &part[idx_start + 1..part.len() - 1]; // strip []
            for s in idx_str.split(',') {
                indices.push(s.trim().parse::<u32>().unwrap());
            }
        }

        // Extended symbol segment 0x91
        buf.push(0x91);
        buf.push(name.len() as u8);
        buf.extend_from_slice(name.as_bytes());
        if name.len() % 2 != 0 {
            buf.push(0x00);
        }

        for idx in indices {
            buf.extend_from_slice(&encode_array_index_segment(idx));
        }

        segments.push(());
    }

    // Prepend size in words
    let size_in_words = buf.len().div_ceil(2) as u8;
    let mut out = Vec::with_capacity(1 + buf.len());
    out.push(size_in_words);
    out.extend_from_slice(&buf);
    out
}

fn encode_array_index_segment(idx: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    if idx > 0xFFFF {
        // 4-byte index: 0x2A, 0x00, UINT32
        buf.push(0x2A);
        buf.push(0x00);
        buf.extend_from_slice(&idx.to_le_bytes());
    } else if idx > 0xFF {
        // 2-byte index: 0x29, 0x00, UINT
        buf.push(0x29);
        buf.push(0x00);
        buf.extend_from_slice(&(idx as u16).to_le_bytes());
    } else {
        // 1-byte index: 0x28, value
        buf.push(0x28);
        buf.push(idx as u8);
    }
    buf
}

/// Build CIP request payload (service + path + data)
pub fn build_cip_request(
    service: CipService,
    tag: &str,
    value: Option<&CipValue>,
    array_size: Option<u16>,
) -> Vec<u8> {
    let mut buf = Vec::new();

    // Service code
    buf.push(service.as_u8());

    // EPATH (symbol path)
    let path = encode_epath(tag);
    buf.extend_from_slice(&path);

    match service {
        CipService::ReadData => {
            // ReadData requires element count (UINT)
            let count = array_size.unwrap_or(1);
            buf.extend_from_slice(&count.to_le_bytes());
        }

        CipService::WriteData => {
            if let Some(v) = value {
                // WriteData requires: type (INT), count (UINT), then data
                let (typ, bytes) = encode_value_bytes(v);
                let count = array_size.unwrap_or(1);

                buf.extend_from_slice(&(typ as u16).to_le_bytes());
                buf.extend_from_slice(&count.to_le_bytes());
                buf.extend_from_slice(&bytes);
            }
        }

        _ => {
            // Other services (fragmented writes, MSP) handled elsewhere
        }
    }

    buf
}

fn encode_value_bytes(value: &CipValue) -> (CipType, Vec<u8>) {
    match value {
        CipValue::Bool(b) => {
            let t = CipType::Bool;
            let v = if *b { 1u8 } else { 0u8 };
            (t, vec![v])
        }
        CipValue::DInt(i) => {
            let t = CipType::DInt;
            (t, i.to_le_bytes().to_vec())
        }
        CipValue::Real(f) => {
            let t = CipType::Real;
            (t, f.to_le_bytes().to_vec())
        }
        CipValue::Int(i) => {
            let t = CipType::Int;
            (t, i.to_le_bytes().to_vec())
        }
        CipValue::SInt(i) => {
            let t = CipType::SInt;
            (t, (*i as u8).to_le_bytes().to_vec())
        }
        CipValue::BitArray(bytes) => {
            let t = CipType::BitArray;
            (t, bytes.clone())
        }
        CipValue::BoolArray(v) => {
            let t = CipType::Bool;
            let bytes: Vec<u8> = v.iter().map(|b| if *b { 1 } else { 0 }).collect();
            (t, bytes)
        }
        CipValue::DIntArray(v) => {
            let t = CipType::DInt;
            let mut bytes = Vec::new();
            for i in v {
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            (t, bytes)
        }
        CipValue::RealArray(v) => {
            let t = CipType::Real;
            let mut bytes = Vec::new();
            for f in v {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            (t, bytes)
        }
        CipValue::IntArray(v) => {
            let t = CipType::Int;
            let mut bytes = Vec::new();
            for i in v {
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            (t, bytes)
        }
        CipValue::SIntArray(v) => {
            let t = CipType::SInt;
            let bytes: Vec<u8> = v.iter().map(|i| *i as u8).collect();
            (t, bytes)
        }
    }
}
pub fn build_cip_write_array(tag: &str, value: &CipValue) -> Vec<u8> {
    let mut buf = Vec::new();

    // Service
    buf.push(CipService::WriteData.as_u8());

    // EPATH
    let path = encode_epath(tag);
    buf.extend_from_slice(&path);

    // Request Data
    let (typ, bytes) = encode_array(value);
    let count = match value {
        CipValue::BoolArray(v) => v.len(),
        CipValue::SIntArray(v) => v.len(),
        CipValue::IntArray(v) => v.len(),
        CipValue::DIntArray(v) => v.len(),
        CipValue::RealArray(v) => v.len(),
        _ => panic!("not an array"),
    } as u16;

    buf.extend_from_slice(&(typ as u16).to_le_bytes());
    buf.extend_from_slice(&count.to_le_bytes());
    buf.extend_from_slice(&bytes);

    buf
}

fn encode_array(value: &CipValue) -> (CipType, Vec<u8>) {
    match value {
        CipValue::BoolArray(v) => {
            let t = CipType::Bool;
            let bytes: Vec<u8> = v.iter().map(|b| if *b { 1 } else { 0 }).collect();
            (t, bytes)
        }
        CipValue::SIntArray(v) => {
            let t = CipType::SInt;
            let bytes: Vec<u8> = v.iter().map(|i| *i as u8).collect();
            (t, bytes)
        }
        CipValue::IntArray(v) => {
            let t = CipType::Int;
            let mut bytes = Vec::new();
            for i in v {
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            (t, bytes)
        }
        CipValue::DIntArray(v) => {
            let t = CipType::DInt;
            let mut bytes = Vec::new();
            for i in v {
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            (t, bytes)
        }
        CipValue::RealArray(v) => {
            let t = CipType::Real;
            let mut bytes = Vec::new();
            for f in v {
                bytes.extend_from_slice(&f.to_le_bytes());
            }
            (t, bytes)
        }
        _ => panic!("encode_array called on non-array value"),
    }
}

/// Decode CIP response payload (type + pad + data)
pub fn decode_cip_response(buf: &[u8]) -> Option<CipValue> {
    if buf.len() < 2 {
        return None;
    }
    let typ = CipType::from_u8(buf[0])?;
    let payload = &buf[2..];
    println!(
        "cip_type = {:?}, payload len = {}, payload = {:02X?}",
        typ,
        payload.len(),
        payload
    );

    Some(match typ {
        CipType::Bool => {
            if payload.len() == 1 {
                CipValue::Bool(payload[0] != 0)
            } else {
                CipValue::BoolArray(payload.iter().map(|b| *b != 0).collect())
            }
        }
        CipType::BitArray => CipValue::BitArray(payload.to_vec()),
        CipType::SInt => {
            if payload.len() == 1 {
                CipValue::SInt(payload[0] as i8)
            } else {
                CipValue::SIntArray(payload.iter().map(|b| *b as i8).collect())
            }
        }
        CipType::Int => {
            if payload.len() == 2 {
                let v = i16::from_le_bytes([payload[0], payload[1]]);
                CipValue::Int(v)
            } else {
                let mut out = Vec::new();
                for chunk in payload.chunks_exact(2) {
                    out.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                }
                CipValue::IntArray(out)
            }
        }
        CipType::DInt => {
            if payload.len() == 4 {
                let v = i32::from_le_bytes(payload[0..4].try_into().unwrap());
                CipValue::DInt(v)
            } else {
                let mut out = Vec::new();
                for chunk in payload.chunks_exact(4) {
                    out.push(i32::from_le_bytes(chunk.try_into().unwrap()));
                }
                CipValue::DIntArray(out)
            }
        }
        CipType::Real => {
            if payload.len() == 4 {
                let v = f32::from_le_bytes(payload[0..4].try_into().unwrap());
                CipValue::Real(v)
            } else {
                let mut out = Vec::new();
                for chunk in payload.chunks_exact(4) {
                    out.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                }
                CipValue::RealArray(out)
            }
        }
    })
}

pub fn build_cip_write_fragment(
    tag: &str,
    value: &CipValue,
    array_size: u16,
    start_index: u32,
    write_count: u16,
) -> Vec<u8> {
    let mut buf = Vec::new();

    // Service
    buf.push(CipService::WriteFragment.as_u8());

    // EPATH
    let path = encode_epath(tag);
    buf.extend_from_slice(&path);

    // Determine type and full bytes for the whole array
    let (typ, full_bytes) = encode_array(value);
    let elem_size = typ.element_size();

    // Compute byte offset and slice
    let byte_offset = (start_index as usize) * elem_size;
    let byte_len = (write_count as usize) * elem_size;
    let slice = &full_bytes[byte_offset..byte_offset + byte_len];

    // Request data:
    // type (INT), total array size (UINT), dataOffset (DINT), then slice
    buf.extend_from_slice(&(typ as u16).to_le_bytes());
    buf.extend_from_slice(&array_size.to_le_bytes());
    buf.extend_from_slice(&(byte_offset as i32).to_le_bytes());
    buf.extend_from_slice(slice);

    buf
}

/// Build a CIP Multiple Service Packet.
/// Each entry in `requests` is a full CIP request payload (service + path + data).
pub fn build_cip_multiple_service_request(requests: &[Vec<u8>]) -> Vec<u8> {
    let mut buf = Vec::new();

    // Service code
    buf.push(CipService::MultipleService.as_u8());

    // Path size = 0 (no path for MSP)
    buf.push(0x00);

    // Number of embedded services
    let count = requests.len() as u16;
    buf.extend_from_slice(&count.to_le_bytes());

    // Offset table (one offset per request)
    let mut offsets = Vec::new();
    let mut current_offset = (2 + count * 2) as usize; // header + offset table

    for req in requests {
        offsets.push(current_offset as u16);
        current_offset += req.len();
    }

    for off in &offsets {
        buf.extend_from_slice(&off.to_le_bytes());
    }

    // Append embedded CIP requests
    for req in requests {
        buf.extend_from_slice(req);
    }

    buf
}
/// Parse a CIP Multiple Service Packet response.
/// Returns a Vec<MultiResult> in the same order as the requests.
pub fn parse_cip_multiple_service_response(buf: &[u8]) -> Vec<MultiResult> {
    let mut out = Vec::new();

    if buf.len() < 4 {
        return out;
    }

    // buf[0] = service | 0x80
    // buf[1] = path size (0)
    // buf[2..4] = number of services
    let count = u16::from_le_bytes([buf[2], buf[3]]) as usize;

    let mut offsets = Vec::new();
    let mut pos = 4;

    for _ in 0..count {
        let off = u16::from_le_bytes([buf[pos], buf[pos + 1]]) as usize;
        offsets.push(off);
        pos += 2;
    }

    // Extract each embedded response
    for i in 0..count {
        let start = offsets[i];
        let end = if i + 1 < count {
            offsets[i + 1]
        } else {
            buf.len()
        };

        let slice = &buf[start..end];

        // CIP response format:
        // [0] = service | 0x80
        // [1] = path size
        // [2..] = path + status + data

        if slice.len() < 4 {
            out.push(MultiResult::Err(0xFF));
            continue;
        }

        let path_size_words = slice[1] as usize;
        let path_bytes = path_size_words * 2;

        let status_offset = 2 + path_bytes;
        if slice.len() <= status_offset {
            out.push(MultiResult::Err(0xFF));
            continue;
        }

        let general_status = slice[status_offset];

        if general_status != 0 {
            out.push(MultiResult::Err(general_status));
            continue;
        }

        // Data begins after: service, path size, path, status, ext status size
        let ext_status_size = slice[status_offset + 1] as usize;
        let data_offset = status_offset + 2 + ext_status_size;

        if slice.len() <= data_offset {
            out.push(MultiResult::Ok(CipValue::DInt(0)));
            continue;
        }

        let data = &slice[data_offset..];

        match decode_cip_response(data) {
            Some(v) => out.push(MultiResult::Ok(v)),
            None => out.push(MultiResult::Err(0xFF)),
        }
    }

    out
}
pub fn encode_port_segment(slot: u8) -> Vec<u8> {
    vec![
        0x01, // Port segment, port = 1 (backplane)
        slot, // Slot number
        0x00, // Padding
    ]
}
pub fn build_cip_request_with_slot(
    service: CipService,
    tag: &str,
    slot: Option<u8>,
    value: Option<&CipValue>,
    count: Option<u16>,
) -> Vec<u8> {
    let mut buf = Vec::new();

    buf.push(service.as_u8());

    let path = encode_epath_with_slot(tag, slot);
    buf.extend_from_slice(&path);

    match service {
        CipService::ReadData => {
            let cnt = count.unwrap_or(1);
            buf.extend_from_slice(&cnt.to_le_bytes());
        }

        CipService::WriteData => {
            if let Some(v) = value {
                let (typ, bytes) = encode_value_bytes(v);
                let cnt = count.unwrap_or(1);

                buf.extend_from_slice(&(typ as u16).to_le_bytes());
                buf.extend_from_slice(&cnt.to_le_bytes());
                buf.extend_from_slice(&bytes);
            }
        }

        _ => {}
    }

    buf
}

pub fn encode_epath_with_slot(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let mut out = Vec::new();

    if let Some(s) = slot {
        out.extend_from_slice(&encode_port_segment(s));
    }

    out.extend_from_slice(&encode_epath(tag));

    out
}
