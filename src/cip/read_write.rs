use crate::cip::epath::encode_epath_with_slot;
use crate::types::{CipType, CipValue};

pub fn build_read_request(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::new();
    cip.push(0x4C); // Read Tag service
    cip.extend(epath);
    cip.extend_from_slice(&1u16.to_le_bytes()); // element count
    cip
}

pub fn build_write_request(tag: &str, value: &CipValue, slot: Option<u8>) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::new();
    cip.push(0x4D); // Write Tag service
    cip.extend(epath);

    match value {
        CipValue::Bool(v) => {
            cip.extend_from_slice(&(CipType::Bool as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.push(if *v { 0xFF } else { 0x00 });
        }
        CipValue::SInt(v) => {
            cip.extend_from_slice(&(CipType::SInt as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.push(*v as u8);
        }
        CipValue::Int(v) => {
            cip.extend_from_slice(&(CipType::Int as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::DInt(v) => {
            cip.extend_from_slice(&(CipType::DInt as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::Real(v) => {
            cip.extend_from_slice(&(CipType::Real as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.extend_from_slice(&v.to_le_bytes());
        }
        CipValue::Unit => {}
    }
    cip
}
