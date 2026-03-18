use std::convert::TryInto;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CipType {
    Bool = 0xC1,
    BitArray = 0xD3,
    SInt = 0xC2,
    Int = 0xC3,
    DInt = 0xC4,
    Real = 0xCA,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CipValue {
    Bool(bool),
    DInt(i32),
    Real(f32),
    Int(i16),
    SInt(i8),
    BitArray(Vec<u8>),
    BoolArray(Vec<bool>),
    DIntArray(Vec<i32>),
    RealArray(Vec<f32>),
    IntArray(Vec<i16>),
    SIntArray(Vec<i8>),
}

impl CipType {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0xC1 => Some(CipType::Bool),
            0xD3 => Some(CipType::BitArray),
            0xC2 => Some(CipType::SInt),
            0xC3 => Some(CipType::Int),
            0xC4 => Some(CipType::DInt),
            0xCA => Some(CipType::Real),
            _ => None,
        }
    }

    pub fn element_size(&self) -> usize {
        match self {
            CipType::Bool => 1,
            CipType::BitArray => 1,
            CipType::SInt => 1,
            CipType::Int => 2,
            CipType::DInt => 4,
            CipType::Real => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub enum MultiResult {
    Ok(CipValue),
    Err(u8),
}

impl CipValue {
    pub fn from_bytes(typ: CipType, data: &[u8]) -> Option<Self> {
        match typ {
            CipType::Bool => Some(CipValue::Bool(*data.first()? != 0)),
            CipType::SInt => Some(CipValue::SInt(*data.first()? as i8)),
            CipType::Int => Some(CipValue::Int(i16::from_le_bytes(
                data.get(0..2)?.try_into().ok()?,
            ))),
            CipType::DInt => Some(CipValue::DInt(i32::from_le_bytes(
                data.get(0..4)?.try_into().ok()?,
            ))),
            CipType::Real => Some(CipValue::Real(f32::from_le_bytes(
                data.get(0..4)?.try_into().ok()?,
            ))),
            _ => None,
        }
    }
}
