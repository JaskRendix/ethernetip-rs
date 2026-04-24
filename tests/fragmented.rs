use ethernetip::cip::decode_cip_data_list;
use ethernetip::CipValue;

#[test]
fn fragmented_two_fragments_dint() {
    // Fragment 1: type ID + first element
    let frag1 = [
        0xC4, 0x00, // type ID (DINT)
        42, 0, 0, 0, // element 0
    ];

    // Fragment 2: second element
    let frag2 = [
        43, 0, 0, 0, // element 1
    ];

    let type_id = u16::from_le_bytes([frag1[0], frag1[1]]);

    let mut combined = Vec::new();
    combined.extend_from_slice(&frag1[2..]);
    combined.extend_from_slice(&frag2);

    let vals = decode_cip_data_list(type_id, &combined);

    assert_eq!(vals, vec![CipValue::DInt(42), CipValue::DInt(43),]);
}

#[test]
fn fragmented_three_fragments_real() {
    let frag1 = [
        0xCA, 0x00, 0x00, 0x00, 0x80, 0x3F, // 1.0
    ];

    let frag2 = [
        0x00, 0x00, 0x00, 0x40, // 2.0
    ];

    let frag3 = [
        0x00, 0x00, 0x40, 0x40, // 3.0
    ];

    let type_id = u16::from_le_bytes([frag1[0], frag1[1]]);

    let mut combined = Vec::new();
    combined.extend_from_slice(&frag1[2..]);
    combined.extend_from_slice(&frag2);
    combined.extend_from_slice(&frag3);

    let vals = decode_cip_data_list(type_id, &combined);

    assert_eq!(
        vals,
        vec![
            CipValue::Real(1.0),
            CipValue::Real(2.0),
            CipValue::Real(3.0),
        ]
    );
}

#[test]
fn decode_empty_payload() {
    let vals = decode_cip_data_list(0x00C4, &[]);
    assert!(vals.is_empty());
}

#[test]
fn decode_unknown_type_id() {
    let vals = decode_cip_data_list(0x9999, &[1, 2, 3, 4]);
    assert!(vals.is_empty());
}

#[test]
fn fragmented_misaligned_payload() {
    // DINT requires 4 bytes per element, but we give 5 bytes
    let vals = decode_cip_data_list(0x00C4, &[1, 0, 0, 0, 99]);
    // Only the first element should decode
    assert_eq!(vals, vec![CipValue::DInt(1)]);
}
