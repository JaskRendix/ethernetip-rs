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

#[test]
fn decode_lint() {
    let buf = [
        0xC5, 0x00, // LINT type
        0x2A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 42i64
    ];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::LInt(42));
}

#[test]
fn decode_string_short() {
    // STRING: len=3, data="ABC"
    let buf = [
        0xD0, 0x00, // STRING type
        0x03, 0x00, // length = 3
        b'A', b'B', b'C',
        // no padding needed for decode
    ];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::String("ABC".into()));
}

#[test]
fn decode_string_empty() {
    let buf = [
        0xD0, 0x00, // STRING type
        0x00, 0x00, // length = 0
    ];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::String(String::new()));
}

#[test]
fn decode_bool_packed_array() {
    // 0b01010101 = [1,0,1,0,1,0,1,0]
    let data = [0b01010101];
    let vals = decode_cip_data_list(0x00D3, &data);

    assert_eq!(
        vals,
        vec![
            CipValue::Bool(true),
            CipValue::Bool(false),
            CipValue::Bool(true),
            CipValue::Bool(false),
            CipValue::Bool(true),
            CipValue::Bool(false),
            CipValue::Bool(true),
            CipValue::Bool(false),
        ]
    );
}

#[test]
fn decode_multi_lint() {
    let data = [42, 0, 0, 0, 0, 0, 0, 0, 43, 0, 0, 0, 0, 0, 0, 0];
    let vals = decode_cip_data_list(0x00C5, &data);
    assert_eq!(vals, vec![CipValue::LInt(42), CipValue::LInt(43)]);
}

#[test]
fn decode_fragmented_lint() {
    let frag1 = [
        0xC5, 0x00, // type ID
        42, 0, 0, 0, 0, 0, 0, 0,
    ];
    let frag2 = [43, 0, 0, 0, 0, 0, 0, 0];

    let type_id = u16::from_le_bytes([frag1[0], frag1[1]]);

    let mut combined = Vec::new();
    combined.extend_from_slice(&frag1[2..]);
    combined.extend_from_slice(&frag2);

    let vals = decode_cip_data_list(type_id, &combined);

    assert_eq!(vals, vec![CipValue::LInt(42), CipValue::LInt(43)]);
}

#[test]
fn decode_fragmented_string() {
    // Fragment 1: type + len + "He"
    let frag1 = [
        0xD0, 0x00, // STRING
        0x05, 0x00, // length = 5
        b'H', b'e',
    ];

    // Fragment 2: "llo"
    let frag2 = [b'l', b'l', b'o'];

    let type_id = u16::from_le_bytes([frag1[0], frag1[1]]);

    let mut combined = Vec::new();
    combined.extend_from_slice(&frag1[2..]);
    combined.extend_from_slice(&frag2);

    let vals = decode_cip_data_list(type_id, &combined);

    assert_eq!(vals, vec![CipValue::String("Hello".into())]);
}
