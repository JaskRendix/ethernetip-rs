use std::convert::TryInto;

pub const COMMAND_LIST_IDENTITY: u16 = 0x0063;
pub const COMMAND_REGISTER_SESSION: u16 = 0x0065;
pub const COMMAND_UNREGISTER_SESSION: u16 = 0x0066;
pub const COMMAND_SEND_RR_DATA: u16 = 0x006F;

#[derive(Debug, Clone)]
pub struct EncapsulationHeader {
    pub command: u16,
    pub length: u16,
    pub session: u32,
    pub status: u32,
    pub sender_context: [u8; 8],
    pub options: u32,
}

impl EncapsulationHeader {
    pub const SIZE: usize = 24;

    pub fn new(command: u16, length: u16, session: u32) -> Self {
        Self {
            command,
            length,
            session,
            status: 0,
            sender_context: [0; 8],
            options: 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 24] {
        let mut buf = [0u8; 24];
        buf[0..2].copy_from_slice(&self.command.to_le_bytes());
        buf[2..4].copy_from_slice(&self.length.to_le_bytes());
        buf[4..8].copy_from_slice(&self.session.to_le_bytes());
        buf[8..12].copy_from_slice(&self.status.to_le_bytes());
        buf[12..20].copy_from_slice(&self.sender_context);
        buf[20..24].copy_from_slice(&self.options.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 24 {
            return None;
        }
        Some(Self {
            command: u16::from_le_bytes(buf[0..2].try_into().ok()?),
            length: u16::from_le_bytes(buf[2..4].try_into().ok()?),
            session: u32::from_le_bytes(buf[4..8].try_into().ok()?),
            status: u32::from_le_bytes(buf[8..12].try_into().ok()?),
            sender_context: buf[12..20].try_into().ok()?,
            options: u32::from_le_bytes(buf[20..24].try_into().ok()?),
        })
    }
}
