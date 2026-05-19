use crate::cip::epath::encode_epath_with_slot;
use crate::cip::service::CipService;
use crate::types::{CipType, CipValue};

#[derive(Clone, Copy, Debug)]
pub enum TransportTrigger {
    Class3ClientInitiated, // 0xA3 (what you use today)
                           // extend later if you want more:
                           // Cyclic,
                           // ChangeOfState,
                           // ApplicationObject,
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

pub fn build_read_request(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::with_capacity(2 + epath.len());
    cip.push(CipService::ReadData as u8);
    cip.extend_from_slice(&epath);
    cip.extend_from_slice(&1u16.to_le_bytes()); // element count = 1
    cip
}

pub fn build_write_request(tag: &str, value: &CipValue, slot: Option<u8>) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::with_capacity(4 + epath.len());
    cip.push(CipService::WriteData as u8);
    cip.extend_from_slice(&epath);

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

        CipValue::LInt(v) => {
            cip.extend_from_slice(&(CipType::LInt as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.extend_from_slice(&v.to_le_bytes());
        }

        CipValue::Real(v) => {
            cip.extend_from_slice(&(CipType::Real as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());
            cip.extend_from_slice(&v.to_le_bytes());
        }

        CipValue::String(s) => {
            // Rockwell STRING: UINT length + SINT[82]
            let bytes = s.as_bytes();
            let len = bytes.len().min(82);

            cip.extend_from_slice(&(CipType::String as u16).to_le_bytes());
            cip.extend_from_slice(&1u16.to_le_bytes());

            cip.extend_from_slice(&(len as u16).to_le_bytes());
            cip.extend_from_slice(&bytes[..len]);

            // pad to 82 bytes
            if len < 82 {
                cip.extend(std::iter::repeat_n(0, 82 - len));
            }
        }

        CipValue::BoolPacked(bytes) => {
            cip.extend_from_slice(&(CipType::BoolPacked as u16).to_le_bytes());
            cip.extend_from_slice(&(bytes.len() as u16).to_le_bytes()); // element count = number of packed bytes
            cip.extend_from_slice(bytes);
        }

        CipValue::Unit => {
            // No payload
        }
    }

    cip
}

pub fn build_read_fragmented_request(
    tag: &str,
    count: u16,
    offset: u32,
    slot: Option<u8>,
) -> Vec<u8> {
    let epath = encode_epath_with_slot(tag, slot);

    let mut cip = Vec::with_capacity(1 + epath.len() + 2 + 4);

    cip.push(CipService::ReadFragmented as u8);
    cip.extend_from_slice(&epath);
    cip.extend_from_slice(&count.to_le_bytes());
    cip.extend_from_slice(&offset.to_le_bytes());

    cip
}

fn encode_connection_manager_path(slot: Option<u8>) -> Vec<u8> {
    let mut segments = Vec::new();

    if let Some(slot) = slot {
        // Port segment: port 1 (backplane), link = slot
        segments.push(0x01); // port
        segments.push(slot); // link address
        segments.push(0x00); // pad
        segments.push(0x00); // pad
    }

    // Class 0x06
    segments.push(0x20);
    segments.push(0x06);

    // Instance 0x01
    segments.push(0x24);
    segments.push(0x01);

    let word_count = (segments.len() / 2) as u8;

    let mut out = Vec::with_capacity(1 + segments.len());
    out.push(word_count);
    out.extend_from_slice(&segments);
    out
}

pub fn build_forward_open_request(slot: Option<u8>, params: ConnectionParams) -> Vec<u8> {
    let path = encode_connection_manager_path(slot);

    let mut cip = Vec::new();
    cip.push(CipService::ForwardOpen as u8);
    cip.extend_from_slice(&path);

    // Priority/Timeout: priority = 0, timeout ticks = 10
    cip.push(0x0A);
    cip.push(0x0A);

    // O->T connection ID (assigned by target for explicit messaging, so 0)
    cip.extend_from_slice(&0u32.to_le_bytes());

    // T->O connection ID (not used for explicit messaging)
    cip.extend_from_slice(&0u32.to_le_bytes());

    // Connection serial number
    cip.extend_from_slice(&1u16.to_le_bytes());

    // Originator vendor ID (0 = non‑registered / generic)
    cip.extend_from_slice(&0u16.to_le_bytes());

    // Originator serial number (arbitrary but stable)
    cip.extend_from_slice(&0x1234_5678u32.to_le_bytes());

    // Timeout multiplier
    cip.push(3);

    // Reserved (3 bytes)
    cip.extend_from_slice(&[0x00, 0x00, 0x00]);

    cip.extend_from_slice(&params.rpi.to_le_bytes());

    let o_to_t_params: u16 = 0x4000 | params.o_to_t_size;
    cip.extend_from_slice(&o_to_t_params.to_le_bytes());

    cip.extend_from_slice(&params.rpi.to_le_bytes()); // or 0 for explicit messaging

    let t_to_o_params: u16 = 0x4000 | params.t_to_o_size;
    cip.extend_from_slice(&t_to_o_params.to_le_bytes());

    // Transport class trigger: Class 3, client‑initiated, application trigger
    cip.push(params.trigger.to_byte());

    // Connection path size (in words) and path:
    // For explicit messaging to the CPU: port 1 backplane, slot, class 0x02 (Message Router), instance 0x01
    let mut conn_path_segments = Vec::new();
    if let Some(slot) = slot {
        conn_path_segments.push(0x01); // port 1
        conn_path_segments.push(slot); // link = slot
        conn_path_segments.push(0x00);
        conn_path_segments.push(0x00);
    }
    conn_path_segments.push(0x20); // class
    conn_path_segments.push(0x02); // Message Router
    conn_path_segments.push(0x24); // instance
    conn_path_segments.push(0x01); // instance 1

    let conn_path_words = (conn_path_segments.len() / 2) as u8;
    cip.push(conn_path_words);
    cip.extend_from_slice(&conn_path_segments);

    cip
}

pub fn build_forward_close_request(slot: Option<u8>) -> Vec<u8> {
    let path = encode_connection_manager_path(slot);

    let mut cip = Vec::new();
    cip.push(CipService::ForwardClose as u8);
    cip.extend_from_slice(&path);

    // Priority/Timeout
    cip.push(0x0A);
    cip.push(0x0A);

    // Connection serial number (must match ForwardOpen)
    cip.extend_from_slice(&1u16.to_le_bytes());

    // Originator vendor ID (must match ForwardOpen)
    cip.extend_from_slice(&0u16.to_le_bytes());

    // Originator serial number (must match ForwardOpen)
    cip.extend_from_slice(&0x1234_5678u32.to_le_bytes());

    // Connection path size and path (same as in ForwardOpen)
    let mut conn_path_segments = Vec::new();
    if let Some(slot) = slot {
        conn_path_segments.push(0x01); // port 1
        conn_path_segments.push(slot); // link = slot
        conn_path_segments.push(0x00);
        conn_path_segments.push(0x00);
    }
    conn_path_segments.push(0x20); // class
    conn_path_segments.push(0x02); // Message Router
    conn_path_segments.push(0x24); // instance
    conn_path_segments.push(0x01); // instance 1

    let conn_path_words = (conn_path_segments.len() / 2) as u8;
    cip.push(conn_path_words);
    cip.extend_from_slice(&conn_path_segments);

    cip
}

pub fn build_large_forward_open_request(slot: Option<u8>, params: ConnectionParams) -> Vec<u8> {
    let path = encode_connection_manager_path(slot);

    let mut cip = Vec::new();
    cip.push(CipService::LargeForwardOpen as u8);
    cip.extend_from_slice(&path);

    // Priority/Timeout
    cip.push(0x0A);
    cip.push(0x0A);

    // O->T connection ID (assigned by target)
    cip.extend_from_slice(&0u32.to_le_bytes());

    // T->O connection ID
    cip.extend_from_slice(&0u32.to_le_bytes());

    // Connection serial number
    cip.extend_from_slice(&1u16.to_le_bytes());

    // Originator vendor ID
    cip.extend_from_slice(&0u16.to_le_bytes());

    // Originator serial number
    cip.extend_from_slice(&0x1234_5678u32.to_le_bytes());

    // Timeout multiplier
    cip.push(3);

    // Reserved (3 bytes)
    cip.extend_from_slice(&[0, 0, 0]);

    // O->T RPI (100 ms)
    cip.extend_from_slice(&params.rpi.to_le_bytes());

    // O->T connection parameters (32‑bit)
    // Example: variable length, 2000 bytes max
    let o_to_t_params: u32 = 0x4000_0000 | params.o_to_t_size as u32;
    cip.extend_from_slice(&o_to_t_params.to_le_bytes());

    // T->O RPI
    cip.extend_from_slice(&0u32.to_le_bytes());

    // T->O connection parameters (32‑bit)
    cip.extend_from_slice(&0u32.to_le_bytes());

    // Transport class trigger
    cip.push(params.trigger.to_byte());

    // Connection path
    let mut conn_path_segments = Vec::new();
    if let Some(slot) = slot {
        conn_path_segments.push(0x01);
        conn_path_segments.push(slot);
        conn_path_segments.push(0x00);
        conn_path_segments.push(0x00);
    }
    conn_path_segments.push(0x20);
    conn_path_segments.push(0x02);
    conn_path_segments.push(0x24);
    conn_path_segments.push(0x01);

    let conn_path_words = (conn_path_segments.len() / 2) as u8;
    cip.push(conn_path_words);
    cip.extend_from_slice(&conn_path_segments);

    cip
}

pub fn build_large_forward_close_request(slot: Option<u8>) -> Vec<u8> {
    let path = encode_connection_manager_path(slot);

    let mut cip = Vec::new();
    cip.push(CipService::LargeForwardClose as u8);
    cip.extend_from_slice(&path);

    // Priority/Timeout
    cip.push(0x0A);
    cip.push(0x0A);

    // Connection serial number (must match ForwardOpen)
    cip.extend_from_slice(&1u16.to_le_bytes());

    // Originator vendor ID
    cip.extend_from_slice(&0u16.to_le_bytes());

    // Originator serial number
    cip.extend_from_slice(&0x1234_5678u32.to_le_bytes());

    // Connection path (same as Large Forward Open)
    let mut conn_path_segments = Vec::new();
    if let Some(slot) = slot {
        conn_path_segments.push(0x01);
        conn_path_segments.push(slot);
        conn_path_segments.push(0x00);
        conn_path_segments.push(0x00);
    }
    conn_path_segments.push(0x20);
    conn_path_segments.push(0x02);
    conn_path_segments.push(0x24);
    conn_path_segments.push(0x01);

    let conn_path_words = (conn_path_segments.len() / 2) as u8;
    cip.push(conn_path_words);
    cip.extend_from_slice(&conn_path_segments);

    cip
}
