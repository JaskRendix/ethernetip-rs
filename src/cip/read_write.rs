use crate::types::{CipType, CipValue};

pub fn build_read_request(tag: &str) -> Vec<u8> {
    let mut path = Vec::new();
    path.push(0x91);
    path.push(tag.len() as u8);
    path.extend_from_slice(tag.as_bytes());
    if !tag.len().is_multiple_of(2) {
        path.push(0x00);
    }

    let mut cip = Vec::new();
    cip.push(0x4C);
    cip.push((path.len() / 2) as u8);
    cip.extend(path);
    cip.extend_from_slice(&1u16.to_le_bytes());
    cip
}

pub fn build_write_request(tag: &str, value: &CipValue) -> Vec<u8> {
    let mut path = Vec::new();
    path.push(0x91);
    path.push(tag.len() as u8);
    path.extend_from_slice(tag.as_bytes());

    let path_words = tag.len().div_ceil(2) as u8;

    if !tag.len().is_multiple_of(2) {
        path.push(0x00);
    }

    let mut cip = Vec::new();
    cip.push(0x4D);
    cip.push(path_words);
    cip.extend_from_slice(&path);

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
