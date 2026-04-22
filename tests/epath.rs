use ethernetip::cip::{encode_epath, encode_epath_with_slot};

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

#[test]
fn encode_large_array_index() {
    let e = encode_epath("Array[256]");
    assert_eq!(
        e,
        vec![
            0x06, 0x91, 0x05, b'A', b'r', b'r', b'a', b'y', 0x00, 0x29, 0x00, 0x00,
            0x01 // 2-byte index, 256 = 0x0100
        ]
    );
}

#[test]
fn encode_dotted_struct() {
    let e = encode_epath("Motor.Speed");
    assert_eq!(
        e,
        vec![
            0x08, 0x91, 0x05, b'M', b'o', b't', b'o', b'r', 0x00, 0x91, 0x05, b'S', b'p', b'e',
            b'e', b'd', 0x00
        ]
    );
}

#[test]
fn encode_multi_index() {
    let e = encode_epath("A[1,2]");
    assert_eq!(
        e,
        vec![0x04, 0x91, 0x01, b'A', 0x00, 0x28, 0x01, 0x28, 0x02]
    );
}

#[test]
fn encode_no_slot() {
    let e = encode_epath_with_slot("Tag", None);
    assert_eq!(e, encode_epath("Tag"));
}

#[test]
fn encode_with_slot() {
    let e = encode_epath_with_slot("Tag", Some(3));
    assert_eq!(
        e,
        vec![
            0x05, // 5 words total
            0x01, 0x03, // port 1, slot 3
            0x00, 0x00, // required padding
            0x91, 0x03, b'T', b'a', b'g', 0x00
        ]
    );
}
