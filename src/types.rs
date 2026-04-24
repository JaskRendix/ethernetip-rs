#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CipType {
    Bool = 0xC1,
    SInt = 0xC2,
    Int = 0xC3,
    DInt = 0xC4,
    LInt = 0xC5,
    Real = 0xCA,
    String = 0xD0,
    BoolPacked = 0xD3,
}

impl CipType {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0xC1 => Some(Self::Bool),
            0xC2 => Some(Self::SInt),
            0xC3 => Some(Self::Int),
            0xC4 => Some(Self::DInt),
            0xC5 => Some(Self::LInt),
            0xCA => Some(Self::Real),
            0xD0 => Some(Self::String),
            0xD3 => Some(Self::BoolPacked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CipValue {
    Bool(bool),
    SInt(i8),
    Int(i16),
    DInt(i32),
    LInt(i64),
    Real(f32),
    String(String),
    Unit,
}

/// Result type used by CIP Multiple Service Packet (MSP) responses.
///
/// - `Ok(T)` contains a successfully decoded CIP value.
/// - `Err(u8)` contains the CIP general status byte.
#[derive(Debug, Clone, PartialEq)]
pub enum MultiResult<T> {
    Ok(T),
    Err(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub typ: CipType,
    pub array_dims: Option<(u16, u16, u16)>, // up to 3D, unused dims = 0
}
