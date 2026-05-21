use crate::cip::connection::{connection_manager_path, message_router_path, ConnectionIds};
use crate::cip::path::{encode_path, PathSegment};
use crate::cip::payload::encode_value;
use crate::cip::service::CipService;
use crate::cip::ConnectionParams;
use crate::types::CipValue;

pub fn build_read_request(tag: &str, slot: Option<u8>) -> Vec<u8> {
    let path = crate::cip::epath::encode_epath_with_slot(tag, slot);

    let mut out = Vec::with_capacity(2 + path.len());
    out.push(CipService::ReadData as u8);
    out.extend_from_slice(&path);
    out.extend_from_slice(&1u16.to_le_bytes());
    out
}

pub fn build_write_request(tag: &str, value: &CipValue, slot: Option<u8>) -> Vec<u8> {
    let path = crate::cip::epath::encode_epath_with_slot(tag, slot);

    let mut out = Vec::new();
    out.push(CipService::WriteData as u8);
    out.extend_from_slice(&path);
    encode_value(value, &mut out);
    out
}

pub fn build_read_fragmented_request(
    tag: &str,
    count: u16,
    offset: u32,
    slot: Option<u8>,
) -> Vec<u8> {
    let path = crate::cip::epath::encode_epath_with_slot(tag, slot);

    let mut out = Vec::new();
    out.push(CipService::ReadFragmented as u8);
    out.extend_from_slice(&path);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
    out
}

pub fn build_get_attribute_single_request(
    class: u16,
    instance: u16,
    attribute: u16,
    slot: Option<u8>,
) -> Vec<u8> {
    let mut segs = Vec::new();

    if let Some(slot) = slot {
        segs.push(PathSegment::Port {
            port: 1,
            link: slot,
        });
    }

    segs.push(PathSegment::Class(class));
    segs.push(PathSegment::Instance(instance));
    segs.push(PathSegment::Attribute(attribute));

    let path = encode_path(&segs);

    let mut out = Vec::new();
    out.push(CipService::GetAttributeSingle as u8);
    out.extend_from_slice(&path);
    out
}

pub fn build_get_attribute_all_request(class: u16, instance: u16, slot: Option<u8>) -> Vec<u8> {
    let mut segs = Vec::new();

    if let Some(slot) = slot {
        segs.push(PathSegment::Port {
            port: 1,
            link: slot,
        });
    }

    segs.push(PathSegment::Class(class));
    segs.push(PathSegment::Instance(instance));

    let path = encode_path(&segs);

    let mut out = Vec::new();
    out.push(CipService::GetAttributeAll as u8);
    out.extend_from_slice(&path);
    out
}

pub fn build_forward_open_request(
    slot: Option<u8>,
    params: ConnectionParams,
    ids: ConnectionIds,
) -> Vec<u8> {
    params.validate().unwrap();

    let cm_path = connection_manager_path(slot);
    let mr_path = message_router_path(slot);

    let mut out = Vec::new();
    out.push(CipService::ForwardOpen as u8);
    out.extend_from_slice(&cm_path);

    out.extend_from_slice(&[0x0A, 0x0A]); // priority/timeout

    out.extend_from_slice(&0u32.to_le_bytes()); // O->T ID
    out.extend_from_slice(&0u32.to_le_bytes()); // T->O ID

    out.extend_from_slice(&ids.serial.to_le_bytes());
    out.extend_from_slice(&ids.vendor.to_le_bytes());
    out.extend_from_slice(&ids.originator_serial.to_le_bytes());

    out.push(3); // timeout multiplier
    out.extend_from_slice(&[0, 0, 0]);

    out.extend_from_slice(&params.rpi.to_le_bytes());

    let o_to_t: u16 = 0x4000 | params.o_to_t_size;
    out.extend_from_slice(&o_to_t.to_le_bytes());

    out.extend_from_slice(&params.rpi.to_le_bytes());

    let t_to_o: u16 = 0x4000 | params.t_to_o_size;
    out.extend_from_slice(&t_to_o.to_le_bytes());

    out.push(params.trigger.to_byte());

    out.push((mr_path.len() / 2) as u8);
    out.extend_from_slice(&mr_path[1..]);

    out
}

pub fn build_forward_close_request(slot: Option<u8>, ids: ConnectionIds) -> Vec<u8> {
    let cm_path = connection_manager_path(slot);
    let mr_path = message_router_path(slot);

    let mut out = Vec::new();
    out.push(CipService::ForwardClose as u8);
    out.extend_from_slice(&cm_path);

    out.extend_from_slice(&[0x0A, 0x0A]); // priority/timeout

    out.extend_from_slice(&ids.serial.to_le_bytes());
    out.extend_from_slice(&ids.vendor.to_le_bytes());
    out.extend_from_slice(&ids.originator_serial.to_le_bytes());

    out.push((mr_path.len() / 2) as u8);
    out.extend_from_slice(&mr_path[1..]);

    out
}

pub fn build_large_forward_open_request(
    slot: Option<u8>,
    params: ConnectionParams,
    ids: ConnectionIds,
) -> Vec<u8> {
    params.validate().unwrap();

    let cm_path = connection_manager_path(slot);
    let mr_path = message_router_path(slot);

    let mut out = Vec::new();
    out.push(CipService::LargeForwardOpen as u8);
    out.extend_from_slice(&cm_path);

    out.extend_from_slice(&[0x0A, 0x0A]);

    out.extend_from_slice(&0u32.to_le_bytes()); // O->T ID
    out.extend_from_slice(&0u32.to_le_bytes()); // T->O ID

    out.extend_from_slice(&ids.serial.to_le_bytes());
    out.extend_from_slice(&ids.vendor.to_le_bytes());
    out.extend_from_slice(&ids.originator_serial.to_le_bytes());

    out.push(3);
    out.extend_from_slice(&[0, 0, 0]);

    out.extend_from_slice(&params.rpi.to_le_bytes());

    let o_to_t: u32 = 0x4000_0000 | params.o_to_t_size as u32;
    out.extend_from_slice(&o_to_t.to_le_bytes());

    out.extend_from_slice(&0u32.to_le_bytes()); // T->O RPI
    out.extend_from_slice(&0u32.to_le_bytes()); // T->O params

    out.push(params.trigger.to_byte());

    out.push((mr_path.len() / 2) as u8);
    out.extend_from_slice(&mr_path[1..]);

    out
}

pub fn build_large_forward_close_request(slot: Option<u8>, ids: ConnectionIds) -> Vec<u8> {
    let cm_path = connection_manager_path(slot);
    let mr_path = message_router_path(slot);

    let mut out = Vec::new();
    out.push(CipService::LargeForwardClose as u8);
    out.extend_from_slice(&cm_path);

    out.extend_from_slice(&[0x0A, 0x0A]);

    out.extend_from_slice(&ids.serial.to_le_bytes());
    out.extend_from_slice(&ids.vendor.to_le_bytes());
    out.extend_from_slice(&ids.originator_serial.to_le_bytes());

    out.push((mr_path.len() / 2) as u8);
    out.extend_from_slice(&mr_path[1..]);

    out
}

pub fn decode_extended_status(res: &[u8]) -> Vec<u16> {
    if res.len() < 4 {
        return Vec::new();
    }

    let count = res[3] as usize;
    let mut out = Vec::with_capacity(count);

    let mut pos = 4;
    for _ in 0..count {
        if pos + 1 >= res.len() {
            break;
        }
        out.push(u16::from_le_bytes([res[pos], res[pos + 1]]));
        pos += 2;
    }

    out
}

pub fn describe_extended_status(words: &[u16]) -> Option<String> {
    if words.is_empty() {
        return None;
    }

    let mut msgs = Vec::new();

    for w in words {
        let msg = match *w {
            0x0100 => "Connection timeout",
            0x0204 => "Invalid connection size",
            0x0205 => "Invalid RPI",
            0x0315 => "Insufficient resources",
            0x0316 => "Unsupported transport trigger",
            _ => return Some(format!("Extended status: 0x{:04X}", w)),
        };
        msgs.push(msg.to_string());
    }

    Some(msgs.join("; "))
}
