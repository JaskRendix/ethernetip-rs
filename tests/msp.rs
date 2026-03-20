use ethernetip::cip::{
    build_cip_multiple_service_request, parse_cip_multiple_service_response, CipService,
};
use ethernetip::types::{CipValue, MultiResult};

fn block_ok(typ: u8, value: &[u8]) -> Vec<u8> {
    let mut v = vec![
        0xCC, 0x00, // service|0x80, reserved
        0x00, // general status OK
        0x00, // ext words = 0
        typ, 0x00, // CIP type + pad
    ];
    v.extend_from_slice(value);
    v
}

fn block_ok_ext(typ: u8, ext: &[u8], value: &[u8]) -> Vec<u8> {
    let mut v = vec![
        0xCC,
        0x00,
        0x00,                  // general status OK
        (ext.len() / 2) as u8, // ext words
    ];
    v.extend_from_slice(ext);
    v.push(typ);
    v.push(0x00);
    v.extend_from_slice(value);
    v
}

fn block_err(status: u8) -> Vec<u8> {
    vec![
        0xCC, 0x00, status, // general status
        0x00,   // ext words
    ]
}

fn make_msp_response(blocks: &[Vec<u8>]) -> Vec<u8> {
    let count = blocks.len() as u16;
    let mut out = Vec::new();

    // MSP header
    out.push(0x8A); // MultipleService | 0x80
    out.push(0x00); // path size = 0
    out.extend_from_slice(&count.to_le_bytes());

    // Offsets
    let mut offsets = Vec::new();
    let mut current = 2 + 2 + (count * 2); // header + offset table
    for b in blocks {
        offsets.push(current);
        current += b.len() as u16;
    }
    for off in offsets {
        out.extend_from_slice(&off.to_le_bytes());
    }

    // Blocks
    for b in blocks {
        out.extend_from_slice(b);
    }

    out
}

#[test]
fn msp_basic() {
    let b1 = block_ok(0xC1, &[0x01]); // BOOL TRUE
    let b2 = block_ok(0xC1, &[0x00]); // BOOL FALSE

    let response = make_msp_response(&[b1, b2]);
    let results = parse_cip_multiple_service_response(&response);
    assert_eq!(results.len(), 2);
}

#[test]
fn msp_ok_ok() {
    let b1 = block_ok(0xC4, &42i32.to_le_bytes());
    let b2 = block_ok(0xC4, &42i32.to_le_bytes());

    let response = make_msp_response(&[b1, b2]);
    let results = parse_cip_multiple_service_response(&response);

    assert_eq!(results.len(), 2);
    assert!(matches!(results[0], MultiResult::Ok(CipValue::DInt(42))));
    assert!(matches!(results[1], MultiResult::Ok(CipValue::DInt(42))));
}

#[test]
fn msp_ok_err() {
    let b1 = block_ok(0xC1, &[0x01]); // BOOL TRUE
    let b2 = block_err(0x05); // PathSegmentError

    let response = make_msp_response(&[b1, b2]);
    let results = parse_cip_multiple_service_response(&response);

    assert_eq!(results.len(), 2);
    assert!(matches!(results[0], MultiResult::Ok(CipValue::Bool(true))));
    assert!(matches!(results[1], MultiResult::Err(0x05)));
}

#[test]
fn msp_general_status_error() {
    let b1 = block_err(0x06); // PathDestinationUnknown

    let response = make_msp_response(&[b1]);
    let results = parse_cip_multiple_service_response(&response);

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], MultiResult::Err(0x06)));
}

#[test]
fn msp_with_extended_status() {
    let ext = [0x34, 0x12];
    let b1 = block_ok_ext(0xC4, &ext, &42i32.to_le_bytes());

    let response = make_msp_response(&[b1]);
    let results = parse_cip_multiple_service_response(&response);

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], MultiResult::Ok(CipValue::DInt(42))));
}

#[test]
fn msp_multiple_types() {
    let b1 = block_ok(0xC1, &[0x01]); // BOOL TRUE
    let b2 = block_ok(0xC3, &123i16.to_le_bytes()); // INT 123
    let b3 = block_ok(0xCA, &10.0f32.to_le_bytes()); // REAL 10.0
    let b4 = block_ok(0xC4, &999i32.to_le_bytes()); // DINT 999

    let response = make_msp_response(&[b1, b2, b3, b4]);
    let results = parse_cip_multiple_service_response(&response);

    assert_eq!(results.len(), 4);
    assert!(matches!(results[0], MultiResult::Ok(CipValue::Bool(true))));
    assert!(matches!(results[1], MultiResult::Ok(CipValue::Int(123))));
    assert!(matches!(results[2], MultiResult::Ok(CipValue::Real(10.0))));
    assert!(matches!(results[3], MultiResult::Ok(CipValue::DInt(999))));
}

#[test]
fn msp_builder_roundtrip_header_and_count() {
    let req1 = vec![CipService::ReadData as u8, 0x00];
    let req2 = vec![CipService::ReadData as u8, 0x00];

    let msp = build_cip_multiple_service_request(&[req1, req2]);

    assert_eq!(msp[0], CipService::MultipleService as u8);
    assert_eq!(msp[1], 0x00); // path size
    assert_eq!(u16::from_le_bytes([msp[2], msp[3]]), 2);
}

#[test]
fn msp_write_response_returns_unit() {
    let write_block = vec![0xCC, 0x00, 0x00, 0x00];
    let response = make_msp_response(&[write_block]);
    let results = parse_cip_multiple_service_response(&response);
    assert!(matches!(results[0], MultiResult::Ok(CipValue::Unit)));
}
