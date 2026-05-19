use std::fmt;

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

#[derive(Debug)]
pub enum ForwardOpenError {
    GeneralStatus(u8),
    ExtendedStatus(Vec<u16>),
    InvalidRpi,
    InvalidSize,
    ResourceUnavailable,
    UnsupportedTrigger,
    Timeout,
    Other(String),
}

impl fmt::Display for ForwardOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ForwardOpenError::GeneralStatus(code) => write!(f, "General status 0x{:02X}", code),

            ForwardOpenError::ExtendedStatus(words) => write!(f, "Extended status {:?}", words),

            ForwardOpenError::InvalidRpi => write!(f, "Invalid RPI"),

            ForwardOpenError::InvalidSize => write!(f, "Invalid connection size"),

            ForwardOpenError::ResourceUnavailable => write!(f, "Resource unavailable"),

            ForwardOpenError::UnsupportedTrigger => write!(f, "Unsupported transport trigger"),

            ForwardOpenError::Timeout => write!(f, "Connection timeout"),

            ForwardOpenError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for ForwardOpenError {
    fn from(err: std::io::Error) -> Self {
        ForwardOpenError::Other(err.to_string())
    }
}

pub fn map_extended_status(words: &[u16]) -> ForwardOpenError {
    if words.is_empty() {
        return ForwardOpenError::ExtendedStatus(vec![]);
    }

    match words[0] {
        0x0100 => ForwardOpenError::Timeout,
        0x0204 => ForwardOpenError::InvalidSize,
        0x0205 => ForwardOpenError::InvalidRpi,
        0x0315 => ForwardOpenError::ResourceUnavailable,
        0x0316 => ForwardOpenError::UnsupportedTrigger,
        w => ForwardOpenError::ExtendedStatus(vec![w]),
    }
}
