use crate::cip::path::{encode_path, PathSegment};

#[derive(Clone, Copy, Debug)]
pub enum TransportTrigger {
    Class3ClientInitiated, // 0xA3
}

impl TransportTrigger {
    pub fn to_byte(self) -> u8 {
        match self {
            TransportTrigger::Class3ClientInitiated => 0xA3,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ConnectionParams {
    pub rpi: u32,
    pub o_to_t_size: u16,
    pub t_to_o_size: u16,
    pub trigger: TransportTrigger,
}

impl Default for ConnectionParams {
    fn default() -> Self {
        Self {
            rpi: 100_000,
            o_to_t_size: 500,
            t_to_o_size: 0,
            trigger: TransportTrigger::Class3ClientInitiated,
        }
    }
}

impl ConnectionParams {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.rpi == 0 {
            return Err("RPI must be > 0");
        }
        if self.o_to_t_size == 0 {
            return Err("O->T size must be > 0");
        }
        if self.o_to_t_size > 4000 {
            return Err("O->T size too large");
        }
        if self.t_to_o_size > 4000 {
            return Err("T->O size too large");
        }
        Ok(())
    }
}

pub struct ConnectionIds {
    pub serial: u16,
    pub vendor: u16,
    pub originator_serial: u32,
}

impl Default for ConnectionIds {
    fn default() -> Self {
        Self {
            serial: 1,
            vendor: 0,
            originator_serial: 0x1234_5678,
        }
    }
}

pub fn connection_manager_path(slot: Option<u8>) -> Vec<u8> {
    let mut segs = Vec::new();

    if let Some(slot) = slot {
        segs.push(PathSegment::Port {
            port: 1,
            link: slot,
        });
    }

    segs.push(PathSegment::Class(0x06)); // Connection Manager
    segs.push(PathSegment::Instance(0x01)); // Instance 1

    encode_path(&segs)
}

pub fn message_router_path(slot: Option<u8>) -> Vec<u8> {
    let mut segs = Vec::new();

    if let Some(slot) = slot {
        segs.push(PathSegment::Port {
            port: 1,
            link: slot,
        });
    }

    segs.push(PathSegment::Class(0x02)); // Message Router
    segs.push(PathSegment::Instance(0x01)); // Instance 1

    encode_path(&segs)
}
