use ethernetip::cip::{decode_cip_data_list, decode_cip_response};
use ethernetip::CipValue;

#[test]
fn decode_bool_true() {
    let buf = [0xC1, 0x00, 0x01];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Bool(true));
}

#[test]
fn decode_bool_false() {
    let buf = [0xC1, 0x00, 0x00];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Bool(false));
}

#[test]
fn decode_sint() {
    let buf = [0xC2, 0x00, 0xFE]; // -2
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::SInt(-2));
}

#[test]
fn decode_int() {
    let buf = [0xC3, 0x00, 0x34, 0x12]; // 0x1234 LE
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Int(0x1234));
}

#[test]
fn decode_dint() {
    let buf = [0xC4, 0x00, 42, 0, 0, 0];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::DInt(42));
}

#[test]
fn decode_real() {
    let buf = [0xCA, 0x00, 0x00, 0x00, 0x20, 0x41];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Real(10.0));
}

#[test]
fn decode_multi_dint() {
    let data = [0x2A, 0x00, 0x00, 0x00, 0x2B, 0x00, 0x00, 0x00];
    let vals = decode_cip_data_list(0x00C4, &data);
    assert_eq!(vals, vec![CipValue::DInt(42), CipValue::DInt(43)]);
}

#[test]
fn decode_multi_bool() {
    let data = [1, 0, 1, 1, 0];
    let vals = decode_cip_data_list(0x00C1, &data);

    assert_eq!(
        vals,
        vec![
            CipValue::Bool(true),
            CipValue::Bool(false),
            CipValue::Bool(true),
            CipValue::Bool(true),
            CipValue::Bool(false),
        ]
    );
}

#[test]
fn decode_fragmented_simulated() {
    // Fragment 1: type ID + first element
    let frag1 = [
        0xC4, 0x00, // type ID
        0x2A, 0x00, 0x00, 0x00, // 42
    ];

    // Fragment 2: second element
    let frag2 = [
        0x2B, 0x00, 0x00, 0x00, // 43
    ];

    let type_id = u16::from_le_bytes([frag1[0], frag1[1]]);

    let mut combined = Vec::new();
    combined.extend_from_slice(&frag1[2..]);
    combined.extend_from_slice(&frag2);

    let vals = decode_cip_data_list(type_id, &combined);

    assert_eq!(vals, vec![CipValue::DInt(42), CipValue::DInt(43)]);
}

#[test]
fn decode_bool_array() {
    let data = [1, 0, 1];
    let vals = decode_cip_data_list(0x00C1, &data);

    assert_eq!(
        vals,
        vec![
            CipValue::Bool(true),
            CipValue::Bool(false),
            CipValue::Bool(true),
        ]
    );
}
