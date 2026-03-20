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
        CipType::Real => {
            let bytes: [u8; 4] = payload.get(0..4)?.try_into().ok()?;
            Some(CipValue::Real(f32::from_le_bytes(bytes)))
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
