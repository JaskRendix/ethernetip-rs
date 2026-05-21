use std::fmt;

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
    pub fn from_u16(w: u16) -> Option<Self> {
        match w {
            0x00C1 => Some(Self::Bool),
            0x00C2 => Some(Self::SInt),
            0x00C3 => Some(Self::Int),
            0x00C4 => Some(Self::DInt),
            0x00C5 => Some(Self::LInt),
            0x00CA => Some(Self::Real),
            0x00D0 => Some(Self::String),
            0x00D3 => Some(Self::BoolPacked),
            _ => None,
        }
    }

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
    BoolPacked(Vec<u8>),
    Unit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredIdentity {
    pub ip: String,
    pub vendor_id: u16,
    pub device_type: u16,
    pub product_code: u16,
    pub revision_major: u8,
    pub revision_minor: u8,
    pub status: u16,
    pub serial: u32,
    pub product_name: String,
}

impl fmt::Display for DiscoveredIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} — {} (Vendor: {}, Type: {}, Code: {}, Rev: {}.{}, Serial: {}, Status: 0x{:04X})",
            self.ip,
            self.product_name,
            self.vendor_id,
            self.device_type,
            self.product_code,
            self.revision_major,
            self.revision_minor,
            self.serial,
            self.status
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentityInfo {
    pub vendor_id: u16,
    pub device_type: u16,
    pub product_code: u16,
    pub revision_major: u8,
    pub revision_minor: u8,
    pub status: u16,
    pub serial_number: u32,
    pub product_name: String,
    pub state: u8,
}

impl IdentityInfo {
    pub fn decode(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 2 + 2 + 2 + 2 + 2 + 4 + 2 + 1 {
            return Err("not enough identity attribute data");
        }

        let mut pos = 0;

        let vendor_id = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;

        let device_type = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;

        let product_code = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;

        let revision_major = data[pos];
        let revision_minor = data[pos + 1];
        pos += 2;

        let status = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;

        let serial_number =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        if data.len() < pos + 2 {
            return Err("identity product name too short");
        }

        let name_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;

        if data.len() < pos + name_len {
            return Err("identity product name length exceeds available data");
        }

        let product_name = String::from_utf8_lossy(&data[pos..pos + name_len]).into_owned();

        let string_block_len = 82;
        if data.len() >= pos + string_block_len {
            pos += string_block_len;
        } else {
            pos += name_len;
        }

        if data.len() <= pos {
            return Err("identity state missing");
        }

        let state = data[pos];

        Ok(Self {
            vendor_id,
            device_type,
            product_code,
            revision_major,
            revision_minor,
            status,
            serial_number,
            product_name,
            state,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionManagerInfo {
    pub revision: u16,
    pub status: u16,
    pub configuration_capability: u16,
}

impl ConnectionManagerInfo {
    pub fn decode(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 6 {
            return Err("not enough connection manager attribute data");
        }

        let revision = u16::from_le_bytes([data[0], data[1]]);
        let status = u16::from_le_bytes([data[2], data[3]]);
        let configuration_capability = u16::from_le_bytes([data[4], data[5]]);

        Ok(Self {
            revision,
            status,
            configuration_capability,
        })
    }
}

impl CipValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            CipValue::Bool(_) => "BOOL",
            CipValue::SInt(_) => "SINT",
            CipValue::Int(_) => "INT",
            CipValue::DInt(_) => "DINT",
            CipValue::LInt(_) => "LINT",
            CipValue::Real(_) => "REAL",
            CipValue::String(_) => "STRING",
            CipValue::BoolPacked(_) => "BOOL_PACKED",
            CipValue::Unit => "UNIT",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_info_decode() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&0x1234u16.to_le_bytes());
        raw.extend_from_slice(&0x5678u16.to_le_bytes());
        raw.extend_from_slice(&0x9ABCu16.to_le_bytes());
        raw.extend_from_slice(&[0x01, 0x02]);
        raw.extend_from_slice(&0x3344u16.to_le_bytes());
        raw.extend_from_slice(&0x11223344u32.to_le_bytes());

        let product_name = b"TestProduct";
        raw.extend_from_slice(&(product_name.len() as u16).to_le_bytes());
        raw.extend_from_slice(product_name);
        raw.extend(std::iter::repeat_n(0, 82 - product_name.len()));

        raw.push(0x05);

        let identity = IdentityInfo::decode(&raw).expect("decode should succeed");
        assert_eq!(identity.vendor_id, 0x1234);
        assert_eq!(identity.device_type, 0x5678);
        assert_eq!(identity.product_code, 0x9ABC);
        assert_eq!(identity.revision_major, 0x01);
        assert_eq!(identity.revision_minor, 0x02);
        assert_eq!(identity.status, 0x3344);
        assert_eq!(identity.serial_number, 0x11223344);
        assert_eq!(identity.product_name, "TestProduct");
        assert_eq!(identity.state, 0x05);
    }

    #[test]
    fn test_connection_manager_info_decode() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&0x0102u16.to_le_bytes());
        raw.extend_from_slice(&0x0304u16.to_le_bytes());
        raw.extend_from_slice(&0x0506u16.to_le_bytes());

        let info = ConnectionManagerInfo::decode(&raw).expect("decode should succeed");
        assert_eq!(info.revision, 0x0102);
        assert_eq!(info.status, 0x0304);
        assert_eq!(info.configuration_capability, 0x0506);
    }

    #[test]
    fn test_ciptype_roundtrip() {
        for (raw, typ) in [
            (0xC1, CipType::Bool),
            (0xC2, CipType::SInt),
            (0xC3, CipType::Int),
            (0xC4, CipType::DInt),
            (0xC5, CipType::LInt),
            (0xCA, CipType::Real),
            (0xD0, CipType::String),
            (0xD3, CipType::BoolPacked),
        ] {
            assert_eq!(CipType::from_u8(raw), Some(typ));
            assert_eq!(CipType::from_u16(raw as u16), Some(typ));
        }
    }

    #[test]
    fn test_identity_info_decode_too_short() {
        let raw = vec![0u8; 5];
        assert!(IdentityInfo::decode(&raw).is_err());
    }

    #[test]
    fn test_identity_info_decode_name_too_short() {
        let mut raw = vec![0u8; 2 + 2 + 2 + 2 + 2 + 4];
        raw.extend_from_slice(&10u16.to_le_bytes()); // claims name length 10
        raw.extend_from_slice(&[1, 2, 3]); // only 3 bytes
        assert!(IdentityInfo::decode(&raw).is_err());
    }

    #[test]
    fn test_connection_manager_info_decode_too_short() {
        let raw = vec![0x01, 0x02, 0x03];
        assert!(ConnectionManagerInfo::decode(&raw).is_err());
    }

    #[test]
    fn test_cipvalue_type_name() {
        assert_eq!(CipValue::Bool(true).type_name(), "BOOL");
        assert_eq!(CipValue::SInt(1).type_name(), "SINT");
        assert_eq!(CipValue::Int(1).type_name(), "INT");
        assert_eq!(CipValue::DInt(1).type_name(), "DINT");
        assert_eq!(CipValue::LInt(1).type_name(), "LINT");
        assert_eq!(CipValue::Real(1.0).type_name(), "REAL");
        assert_eq!(CipValue::String("x".into()).type_name(), "STRING");
        assert_eq!(CipValue::BoolPacked(vec![1]).type_name(), "BOOL_PACKED");
        assert_eq!(CipValue::Unit.type_name(), "UNIT");
    }

    #[test]
    fn test_multiresult_variants() {
        let ok = MultiResult::Ok(123);
        let err: MultiResult<()> = MultiResult::Err(0x05);

        assert!(matches!(ok, MultiResult::Ok(_)));
        assert!(matches!(err, MultiResult::Err(0x05)));
    }

    #[test]
    fn test_symbolinfo_basic() {
        let s = SymbolInfo {
            name: "Tag".into(),
            typ: CipType::DInt,
            array_dims: Some((10, 0, 0)),
        };

        assert_eq!(s.name, "Tag");
        assert_eq!(s.typ, CipType::DInt);
        assert_eq!(s.array_dims, Some((10, 0, 0)));
    }
    #[test]
    fn test_identity_info_decode_exact_boundary() {
        let mut raw = Vec::new();
        raw.extend_from_slice(&1u16.to_le_bytes()); // vendor
        raw.extend_from_slice(&2u16.to_le_bytes()); // type
        raw.extend_from_slice(&3u16.to_le_bytes()); // code
        raw.extend_from_slice(&[1, 2]); // revision
        raw.extend_from_slice(&4u16.to_le_bytes()); // status
        raw.extend_from_slice(&5u32.to_le_bytes()); // serial

        let name = b"A";
        raw.extend_from_slice(&(name.len() as u16).to_le_bytes());
        raw.extend_from_slice(name);

        raw.extend(std::iter::repeat_n(0, 82 - name.len()));
        raw.push(7); // state

        let info = IdentityInfo::decode(&raw).unwrap();
        assert_eq!(info.product_name, "A");
        assert_eq!(info.state, 7);
    }
}
