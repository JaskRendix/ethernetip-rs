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

#[test]
fn decode_sint_positive() {
    let buf = [0xC2, 0x00, 0x7F]; // 127
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::SInt(127));
}

#[test]
fn decode_int_negative() {
    let buf = [0xC3, 0x00, 0xFF, 0xFF]; // -1
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Int(-1));
}

#[test]
fn decode_lint_large() {
    let buf = [
        0xC5, 0x00, // type ID
        0x00, 0x28, 0x6B, 0xEE, 0x09, 0x00, 0x00, 0x00,
    ];

    assert_eq!(buf.len(), 10); // sanity check
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::LInt(42654705664));
}

#[test]
fn decode_real_negative() {
    let v = -1.5f32;
    let mut buf = vec![0xCA, 0x00];
    buf.extend_from_slice(&v.to_le_bytes());
    let decoded = decode_cip_response(&buf).unwrap();
    assert_eq!(decoded, CipValue::Real(-1.5));
}

#[test]
fn decode_multi_sint() {
    let data = [0xFE, 0x01, 0x7F]; // -2, 1, 127
    let vals = decode_cip_data_list(0x00C2, &data);
    assert_eq!(
        vals,
        vec![CipValue::SInt(-2), CipValue::SInt(1), CipValue::SInt(127)]
    );
}

#[test]
fn decode_multi_int() {
    let data = [0x34, 0x12, 0xFF, 0xFF]; // 0x1234, -1
    let vals = decode_cip_data_list(0x00C3, &data);
    assert_eq!(vals, vec![CipValue::Int(0x1234), CipValue::Int(-1)]);
}

#[test]
fn decode_multi_real() {
    let mut data = Vec::new();
    data.extend_from_slice(&1.5f32.to_le_bytes());
    data.extend_from_slice(&2.5f32.to_le_bytes());
    let vals = decode_cip_data_list(0x00CA, &data);
    assert_eq!(vals, vec![CipValue::Real(1.5), CipValue::Real(2.5)]);
}

#[test]
fn decode_multi_string() {
    // Two 84-byte Rockwell strings
    let mut data = Vec::new();
    let s1 = b"Hi";
    data.extend_from_slice(&(s1.len() as u16).to_le_bytes());
    data.extend_from_slice(s1);
    data.extend(std::iter::repeat_n(0u8, 82 - s1.len()));
    let s2 = b"Bye";
    data.extend_from_slice(&(s2.len() as u16).to_le_bytes());
    data.extend_from_slice(s2);
    data.extend(std::iter::repeat_n(0u8, 82 - s2.len()));
    let vals = decode_cip_data_list(0x00D0, &data);
    assert_eq!(
        vals,
        vec![
            CipValue::String("Hi".into()),
            CipValue::String("Bye".into()),
        ]
    );
}

#[test]
fn decode_bool_packed_two_bytes() {
    let data = [0b11110000, 0b00001111];
    let vals = decode_cip_data_list(0x00D3, &data);
    assert_eq!(vals.len(), 16);
    // first byte: bits 0-3 false, bits 4-7 true
    assert_eq!(vals[0], CipValue::Bool(false));
    assert_eq!(vals[4], CipValue::Bool(true));
    // second byte: bits 0-3 true, bits 4-7 false
    assert_eq!(vals[8], CipValue::Bool(true));
    assert_eq!(vals[12], CipValue::Bool(false));
}

#[test]
fn decode_cip_response_too_short() {
    assert!(decode_cip_response(&[]).is_none());
    assert!(decode_cip_response(&[0xC4]).is_none());
}

#[test]
fn decode_cip_response_unknown_type() {
    let buf = [0xFF, 0x00, 0x00, 0x00];
    assert!(decode_cip_response(&buf).is_none());
}

#[test]
fn decode_cip_data_list_unknown_type_returns_empty() {
    let vals = decode_cip_data_list(0xFFFF, &[0x01, 0x02, 0x03, 0x04]);
    assert!(vals.is_empty());
}

#[test]
fn decode_cip_data_list_truncated_dint() {
    // 3 bytes instead of 4 — chunks_exact should skip it
    let data = [0x2A, 0x00, 0x00];
    let vals = decode_cip_data_list(0x00C4, &data);
    assert!(vals.is_empty());
}

#[test]
fn decode_bool_packed_response_returns_none() {
    // BoolPacked via decode_cip_response should return None (use read_array instead)
    let buf = [0xD3, 0x00, 0b01010101];
    assert!(decode_cip_response(&buf).is_none());
}
