#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CipError {
    ConnectionFailure,
    ResourceUnavailable,
    InvalidAttribute,
    PathSegmentError,
    PathDestinationUnknown,
    VendorSpecific(u8),
}

impl From<u8> for CipError {
    fn from(code: u8) -> Self {
        match code {
            0x01 => CipError::ConnectionFailure,
            0x02 => CipError::ResourceUnavailable,
            0x04 => CipError::InvalidAttribute,
            0x05 => CipError::PathSegmentError,
            0x06 => CipError::PathDestinationUnknown,
            other => CipError::VendorSpecific(other),
        }
    }
}
