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
    match type_id {
        0x00C1 => decode_bool_bytes(data),   // BOOL (byte)
        0x00D3 => decode_bool_packed(data),  // BOOL[] packed bits
        0x00C2 => decode_sint_bytes(data),   // SINT
        0x00C3 => decode_int_bytes(data),    // INT
        0x00C4 => decode_dint_bytes(data),   // DINT
        0x00C5 => decode_lint_bytes(data),   // LINT
        0x00CA => decode_real_bytes(data),   // REAL
        0x00D0 => decode_string_bytes(data), // STRING
        _ => Vec::new(),
    }
}

/// BOOL (byte-per-element)
fn decode_bool_bytes(data: &[u8]) -> Vec<CipValue> {
    data.iter().map(|b| CipValue::Bool(*b != 0)).collect()
}

/// BOOL[] packed (Rockwell type 0xD3)
fn decode_bool_packed(data: &[u8]) -> Vec<CipValue> {
    let mut out = Vec::new();
    for byte in data {
        for bit in 0..8 {
            let v = (byte >> bit) & 1 != 0;
            out.push(CipValue::Bool(v));
        }
    }
    out
}

/// SINT
fn decode_sint_bytes(data: &[u8]) -> Vec<CipValue> {
    data.iter().map(|b| CipValue::SInt(*b as i8)).collect()
}

/// INT
fn decode_int_bytes(data: &[u8]) -> Vec<CipValue> {
    data.chunks_exact(2)
        .map(|c| CipValue::Int(i16::from_le_bytes([c[0], c[1]])))
        .collect()
}

/// DINT
fn decode_dint_bytes(data: &[u8]) -> Vec<CipValue> {
    data.chunks_exact(4)
        .map(|c| CipValue::DInt(i32::from_le_bytes([c[0], c[1], c[2], c[3]])))
        .collect()
}

/// LINT (new)
fn decode_lint_bytes(data: &[u8]) -> Vec<CipValue> {
    data.chunks_exact(8)
        .map(|c| {
            CipValue::LInt(i64::from_le_bytes([
                c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7],
            ]))
        })
        .collect()
}

/// REAL
fn decode_real_bytes(data: &[u8]) -> Vec<CipValue> {
    data.chunks_exact(4)
        .map(|c| CipValue::Real(f32::from_le_bytes([c[0], c[1], c[2], c[3]])))
        .collect()
}

/// STRING (Rockwell STRING)
/// Layout:
///   UINT length
///   SINT data[82]
fn decode_string_bytes(data: &[u8]) -> Vec<CipValue> {
    const ROCKWELL_STRING_SIZE: usize = 84;
    let mut out = Vec::new();
    let mut pos = 0;

    while pos + 2 <= data.len() {
        let len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        let str_bytes = &data[pos + 2..data.len().min(pos + 2 + len)];
        out.push(CipValue::String(
            String::from_utf8_lossy(str_bytes).into_owned(),
        ));
        pos += ROCKWELL_STRING_SIZE;
    }

    out
}

/// Decode a standard CIP single-read response.
pub fn decode_cip_response(buf: &[u8]) -> Option<CipValue> {
    if buf.len() < 2 {
        return None;
    }
    let type_id = u16::from_le_bytes([buf[0], buf[1]]);
    if type_id == 0x00D3 {
        return None;
    }
    let data = &buf[2..];
    decode_cip_data_list(type_id, data).into_iter().next()
}

/// Returns the raw packed bytes as a single CipValue::BoolPacked.
/// Use this when you intend to write the value back unchanged.
pub fn decode_cip_data_packed(type_id: u16, data: &[u8]) -> Option<CipValue> {
    if type_id == 0x00D3 {
        Some(CipValue::BoolPacked(data.to_vec()))
    } else {
        None
    }
}
