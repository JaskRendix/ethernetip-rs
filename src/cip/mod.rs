pub mod epath;
pub mod error;
pub mod msp;
pub mod read_write;
pub mod service;
pub mod symbol;
pub mod value;

use crate::types::CipValue;
pub use epath::*;
pub use error::*;
pub use msp::*;
pub use read_write::*;
pub use service::*;
pub use symbol::*;

/// Decode a raw byte buffer into a list of CipValues based on a Type ID.
pub fn decode_cip_data_list(type_id: u16, data: &[u8]) -> Vec<CipValue> {
    let element_size = match type_id {
        0x00C1 => 1, // BOOL
        0x00C2 => 1, // SINT
        0x00C3 => 2, // INT
        0x00C4 => 4, // DINT
        0x00CA => 4, // REAL
        _ => return Vec::new(),
    };

    let mut out = Vec::new();

    for chunk in data.chunks_exact(element_size) {
        let val = match type_id {
            0x00C1 => CipValue::Bool(chunk[0] != 0),
            0x00C2 => CipValue::SInt(chunk[0] as i8),
            0x00C3 => CipValue::Int(i16::from_le_bytes([chunk[0], chunk[1]])),
            0x00C4 => CipValue::DInt(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])),
            0x00CA => CipValue::Real(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])),
            _ => continue,
        };
        out.push(val);
    }

    out
}

/// Decode a standard CIP single-read response.
pub fn decode_cip_response(buf: &[u8]) -> Option<CipValue> {
    if buf.len() < 2 {
        return None;
    }

    let type_id = u16::from_le_bytes([buf[0], buf[1]]);
    let data = &buf[2..];

    decode_cip_data_list(type_id, data).into_iter().next()
}
