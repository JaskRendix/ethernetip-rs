#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CipType {
    Bool = 0xC1,
    SInt = 0xC2,
    Int  = 0xC3,
    DInt = 0xC4,
    Real = 0xCA,
}

impl CipType {
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0xC1 => Some(Self::Bool),
            0xC2 => Some(Self::SInt),
            0xC3 => Some(Self::Int),
            0xC4 => Some(Self::DInt),
            0xCA => Some(Self::Real),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum CipValue {
    Bool(bool),
    SInt(i8),
    Int(i16),
    DInt(i32),
    Real(f32),
}
