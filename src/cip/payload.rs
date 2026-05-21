use crate::types::{CipType, CipValue};

pub fn encode_value(value: &CipValue, out: &mut Vec<u8>) {
    match value {
        CipValue::Bool(v) => {
            out.extend_from_slice(&(CipType::Bool as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.push(if *v { 0xFF } else { 0x00 });
        }
        CipValue::SInt(v) => {
            out.extend_from_slice(&(CipType::SInt as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.push(*v as u8);
        }
        CipValue::Int(v) => {
            out.extend_from_slice(&(CipType::Int as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::DInt(v) => {
            out.extend_from_slice(&(CipType::DInt as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::LInt(v) => {
            out.extend_from_slice(&(CipType::LInt as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::Real(v) => {
            out.extend_from_slice(&(CipType::Real as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::String(s) => {
            let bytes = s.as_bytes();
            let len = bytes.len().min(82);

            out.extend_from_slice(&(CipType::String as u16).to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&(len as u16).to_le_bytes());
            out.extend_from_slice(&bytes[..len]);

            if len < 82 {
                out.extend(std::iter::repeat_n(0, 82 - len));
            }
        }
        CipValue::BoolPacked(bytes) => {
            out.extend_from_slice(&(CipType::BoolPacked as u16).to_le_bytes());
            out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(bytes);
        }
        CipValue::Unit => {}
    }
}
