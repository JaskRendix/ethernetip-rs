use crate::types::{CipType, CipValue};
use std::convert::TryInto;

fn decode_cip_payload(typ: CipType, payload: &[u8]) -> Option<CipValue> {
    match typ {
        CipType::Bool => Some(CipValue::Bool(payload.first().copied()? != 0)),

        CipType::SInt => Some(CipValue::SInt(payload.first().copied()? as i8)),

        CipType::Int => {
            let bytes: [u8; 2] = payload.get(0..2)?.try_into().ok()?;
            Some(CipValue::Int(i16::from_le_bytes(bytes)))
        }

        CipType::DInt => {
            let bytes: [u8; 4] = payload.get(0..4)?.try_into().ok()?;
            Some(CipValue::DInt(i32::from_le_bytes(bytes)))
        }

        CipType::LInt => {
            let bytes: [u8; 8] = payload.get(0..8)?.try_into().ok()?;
            Some(CipValue::LInt(i64::from_le_bytes(bytes)))
        }

        CipType::Real => {
            let bytes: [u8; 4] = payload.get(0..4)?.try_into().ok()?;
            Some(CipValue::Real(f32::from_le_bytes(bytes)))
        }

        CipType::String => {
            if payload.len() < 2 {
                return None;
            }
            let len = u16::from_le_bytes([payload[0], payload[1]]) as usize;
            let bytes = payload.get(2..2 + len)?;
            Some(CipValue::String(
                String::from_utf8_lossy(bytes).into_owned(),
            ))
        }

        CipType::BoolPacked => {
            let mut out = Vec::new();
            for byte in payload {
                for bit in 0..8 {
                    out.push(CipValue::Bool(((byte >> bit) & 1) != 0));
                }
            }
            // For single-read, return only the first element
            out.into_iter().next()
        }
    }
}

pub fn decode_cip_response(buf: &[u8]) -> Option<CipValue> {
    if buf.len() < 2 {
        return None;
    }

    let typ = CipType::from_u8(buf[0])?;
    let pad = buf[1];

    if pad != 0x00 {
        return None;
    }

    let payload = &buf[2..];
    decode_cip_payload(typ, payload)
}
