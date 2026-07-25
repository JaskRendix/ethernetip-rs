use crate::cip::path::{encode_path, PathSegment};
use std::sync::atomic::{AtomicU16, Ordering};

static NEXT_SERIAL: AtomicU16 = AtomicU16::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportTrigger {
    /// Class 3 Client-Initiated (0xA3)
    Class3ClientInitiated = 0xA3,
    /// Cyclic (0x01)
    Cyclic = 0x01,
    /// Change of State (0x02)
    ChangeOfState = 0x02,
}

impl TransportTrigger {
    pub const fn to_byte(self) -> u8 {
        self as u8
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
    pub const fn new(rpi: u32, o_to_t_size: u16, t_to_o_size: u16) -> Self {
        Self {
            rpi,
            o_to_t_size,
            t_to_o_size,
            trigger: TransportTrigger::Class3ClientInitiated,
        }
    }

    pub const fn with_trigger(mut self, trigger: TransportTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.rpi == 0 {
            return Err("RPI must be > 0");
        }
        if self.o_to_t_size == 0 {
            return Err("O->T size must be > 0");
        }
        if self.o_to_t_size > 4000 {
            return Err("O->T size too large (> 4000 bytes)");
        }
        if self.t_to_o_size > 4000 {
            return Err("T->O size too large (> 4000 bytes)");
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
            serial: NEXT_SERIAL.fetch_add(1, Ordering::Relaxed),
            vendor: 0x0013, // Rockwell Automation vendor ID placeholder
            originator_serial: 0x1234_5678,
        }
    }
}

/// Helper to inject backplane slot routing port segments into an EPATH vector.
fn push_slot_routing(segs: &mut Vec<PathSegment>, slot: Option<u8>) {
    if let Some(slot) = slot {
        segs.push(PathSegment::Port {
            port: 1, // Backplane port
            link: slot,
        });
    }
}

pub fn connection_manager_path(slot: Option<u8>) -> Vec<u8> {
    let mut segs = Vec::new();
    push_slot_routing(&mut segs, slot);
    segs.push(PathSegment::Class(0x06)); // Connection Manager
    segs.push(PathSegment::Instance(0x01)); // Instance 1
    encode_path(&segs)
}

pub fn message_router_path(slot: Option<u8>) -> Vec<u8> {
    let mut segs = Vec::new();
    push_slot_routing(&mut segs, slot);
    segs.push(PathSegment::Class(0x02)); // Message Router
    segs.push(PathSegment::Instance(0x01)); // Instance 1
    encode_path(&segs)
}
