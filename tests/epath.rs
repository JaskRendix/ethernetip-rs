use ethernetip::cip::encode_epath;

#[test]
fn encode_simple_tag() {
    let e = encode_epath("Test");
    assert_eq!(
        e,
        vec![
            0x03, // size in words
            0x91, 0x04, // ANSI extended symbol
            b'T', b'e', b's', b't'
        ]
    );
}

#[test]
fn encode_odd_length_tag() {
    let e = encode_epath("Abc");
    assert_eq!(
        e,
        vec![
            0x03, // size in words (correct)
            0x91, 0x03, b'A', b'b', b'c', 0x00
        ]
    );
}

#[test]
fn encode_array_index() {
    let e = encode_epath("Array[5]");
    assert_eq!(
        e,
        vec![
            0x05, // size in words (correct)
            0x91, 0x05, b'A', b'r', b'r', b'a', b'y', 0x00, // padding
            0x28, 0x05 // 1‑byte array index segment
        ]
    );
}
