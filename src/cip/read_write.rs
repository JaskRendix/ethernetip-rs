use crate::cip::epath::encode_epath_with_slot;
use crate::cip::service::CipService;
use crate::types::{CipType, CipValue};

pub fn build_read_request(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::with_capacity(2 + epath.len());
    cip.push(0x4C); // Read Tag service
    cip.extend_from_slice(&epath);
    cip.extend_from_slice(&1u16.to_le_bytes()); // element count = 1
    cip
}

pub fn build_write_request(tag: &str, value: &CipValue, slot: Option<u8>) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::with_capacity(4 + epath.len());
    cip.push(0x4D); // Write Tag service
    cip.extend_from_slice(&epath);

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
        CipValue::Unit => {
            // No payload
        }
    }

    cip
}

pub fn build_read_fragmented_request(
    tag: &str,
    count: u16,
    offset: u32,
    slot: Option<u8>,
) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::with_capacity(1 + epath.len() + 2 + 4);

    cip.push(CipService::ReadFragmented as u8);
    cip.extend_from_slice(&epath);
    cip.extend_from_slice(&count.to_le_bytes());
    cip.extend_from_slice(&offset.to_le_bytes());

    cip
}
