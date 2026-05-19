use ethernetip::cip::{
    build_forward_close_request, build_forward_open_request, build_large_forward_close_request,
    build_large_forward_open_request, build_read_fragmented_request, build_read_request,
    build_write_request, service::CipService, ConnectionParams,
};
use ethernetip::types::{CipType, CipValue};

fn parse_epath_len(cip: &[u8]) -> usize {
    let words = cip[1] as usize;
    2 + words * 2
}

#[test]
fn test_build_read_request() {
    let cip = build_read_request("MyTag", None);
    assert_eq!(cip[0], CipService::ReadData as u8);

    let epath_end = parse_epath_len(&cip);
    let count = u16::from_le_bytes([cip[epath_end], cip[epath_end + 1]]);
    assert_eq!(count, 1);
}

#[test]
fn test_build_write_request_dint() {
    let cip = build_write_request("MyTag", &CipValue::DInt(12345), None);
    assert_eq!(cip[0], CipService::WriteData as u8);

    let epath_end = parse_epath_len(&cip);

    let type_id = u16::from_le_bytes([cip[epath_end], cip[epath_end + 1]]);
    assert_eq!(type_id, CipType::DInt as u16);

    let count = u16::from_le_bytes([cip[epath_end + 2], cip[epath_end + 3]]);
    assert_eq!(count, 1);

    let value = i32::from_le_bytes([
        cip[epath_end + 4],
        cip[epath_end + 5],
        cip[epath_end + 6],
        cip[epath_end + 7],
    ]);
    assert_eq!(value, 12345);
}

#[test]
fn test_build_read_fragmented_request() {
    let cip = build_read_fragmented_request("MyTag", 100, 200, None);
    assert_eq!(cip[0], CipService::ReadFragmented as u8);

    let epath_end = parse_epath_len(&cip);

    let count = u16::from_le_bytes([cip[epath_end], cip[epath_end + 1]]);
    assert_eq!(count, 100);

    let offset = u32::from_le_bytes([
        cip[epath_end + 2],
        cip[epath_end + 3],
        cip[epath_end + 4],
        cip[epath_end + 5],
    ]);
    assert_eq!(offset, 200);
}

#[test]
fn test_forward_open_request_structure() {
    let cip = build_forward_open_request(Some(2), ConnectionParams::default());
    assert_eq!(cip[0], CipService::ForwardOpen as u8);

    let epath_end = parse_epath_len(&cip);

    let idx = epath_end
        + 2  // priority/timeout
        + 4  // O->T ID
        + 4  // T->O ID
        + 2  // serial
        + 2  // vendor
        + 4  // originator serial
        + 1  // timeout multiplier
        + 3  // reserved
        + 4  // O->T RPI
        + 2  // O->T params
        + 4  // T->O RPI
        + 2; // T->O params

    let tct = cip[idx];
    assert_eq!(tct, 0xA3);
}

#[test]
fn test_large_forward_open_request_structure() {
    let cip = build_large_forward_open_request(Some(3), ConnectionParams::default());
    assert_eq!(cip[0], CipService::LargeForwardOpen as u8);

    let epath_end = parse_epath_len(&cip);

    let params_index = epath_end
        + 2  // priority/timeout
        + 4  // O->T ID
        + 4  // T->O ID
        + 2  // serial
        + 2  // vendor
        + 4  // originator serial
        + 1  // timeout multiplier
        + 3  // reserved
        + 4; // O->T RPI

    let params = u32::from_le_bytes([
        cip[params_index],
        cip[params_index + 1],
        cip[params_index + 2],
        cip[params_index + 3],
    ]);

    assert!(params & 0x4000_0000 != 0);
}

#[test]
fn test_forward_close_request() {
    let cip = build_forward_close_request(Some(1));
    assert_eq!(cip[0], CipService::ForwardClose as u8);

    let epath_end = parse_epath_len(&cip);

    let serial = u16::from_le_bytes([cip[epath_end + 2], cip[epath_end + 3]]);
    assert_eq!(serial, 1);
}

#[test]
fn test_large_forward_close_request() {
    let cip = build_large_forward_close_request(Some(1));
    assert_eq!(cip[0], CipService::LargeForwardClose as u8);

    let epath_end = parse_epath_len(&cip);

    let serial = u16::from_le_bytes([cip[epath_end + 2], cip[epath_end + 3]]);
    assert_eq!(serial, 1);
}

#[test]
fn test_forward_open_slot_routing() {
    let cip = build_forward_open_request(Some(5), ConnectionParams::default());
    assert_eq!(cip[2], 0x01);
    assert_eq!(cip[3], 5);
}

#[test]
fn test_connection_manager_path_no_slot() {
    let cip = build_forward_open_request(None, ConnectionParams::default());
    assert_eq!(cip[2], 0x20);
    assert_eq!(cip[3], 0x06);
}

#[test]
fn test_forward_open_custom_params() {
    let params = ConnectionParams {
        rpi: 250_000,
        o_to_t_size: 123,
        t_to_o_size: 77,
    };

    let cip = build_forward_open_request(Some(1), params);

    let epath_end = parse_epath_len(&cip);

    // RPI is after:
    // 2  priority/timeout
    // 4  O->T ID
    // 4  T->O ID
    // 2  serial
    // 2  vendor
    // 4  originator serial
    // 1  timeout multiplier
    // 3  reserved
    let rpi_offset = epath_end + 2 + 4 + 4 + 2 + 2 + 4 + 1 + 3;

    let rpi = u32::from_le_bytes([
        cip[rpi_offset],
        cip[rpi_offset + 1],
        cip[rpi_offset + 2],
        cip[rpi_offset + 3],
    ]);
    assert_eq!(rpi, 250_000);

    // O->T params follow immediately after RPI
    let o_to_t_offset = rpi_offset + 4;

    let o_to_t = u16::from_le_bytes([cip[o_to_t_offset], cip[o_to_t_offset + 1]]);
    assert_eq!(o_to_t & 0x1FFF, 123);
}
