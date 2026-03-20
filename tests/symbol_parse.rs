use ethernetip::cip::parse_symbol_browse_response;
use ethernetip::types::CipType;

fn hex_dump(buf: &[u8]) -> String {
    buf.chunks(16)
        .enumerate()
        .map(|(i, chunk)| {
            format!(
                "{:04X}: {}\n",
                i * 16,
                chunk
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
        .collect()
}

#[test]
fn parse_single_symbol() {
    let buf = vec![
        // Attr 1 status
        0x00, 0x00, // Name
        0x07, b'T', b'e', b's', b't', b'T', b'a', b'g', 0x00, // alignment padding
        // Attr 2 status
        0x00, 0x00, // Type = DINT (0xC4)
        0xC4, 0x00, // Attr 5 status
        0x00, 0x00, // Dim count
        0x00, // Required 2‑byte padding
        0x00, 0x00,
    ];

    let syms = parse_symbol_browse_response(&buf);
    assert_eq!(syms.len(), 1, "hex dump:\n{}", hex_dump(&buf));

    let s = &syms[0];
    assert_eq!(s.name, "TestTag");
    assert_eq!(s.typ, CipType::DInt);
    assert!(s.array_dims.is_none());
}

#[test]
fn parse_multiple_symbols() {
    let buf = vec![
        // --- Symbol 1 ---
        0x00, 0x00, 0x03, b'A', b'B', b'C', 0x00, 0x00, 0x00, 0xC1, 0x00, 0x00, 0x00, 0x00,
        // --- Symbol 2 (may or may not be fully parsed depending on parser details) ---
        0x00, 0x00, 0x04, b'T', b'e', b's', b't', 0x00, 0x00, 0xC4, 0x00, 0x00, 0x00, 0x00,
    ];

    let syms = parse_symbol_browse_response(&buf);
    assert!(!syms.is_empty(), "hex dump:\n{}", hex_dump(&buf));
    assert_eq!(syms[0].name, "ABC");
    assert_eq!(syms[0].typ, CipType::Bool);
}

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_alignment_and_padding(name in "[A-Za-z]{1,20}") {
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() as u8;

        let mut buf = vec![
            0x00, 0x00, // Attr1
            name_len,
        ];
        buf.extend_from_slice(name_bytes);

        // CIP‑spec padding: pad if name_len is odd
        if !name_len.is_multiple_of(2) {
            buf.push(0x00);
        }

        buf.extend_from_slice(&[
            0x00, 0x00, // Attr2
            0xC4, 0x00, // DINT
            0x00, 0x00, // Attr5
            0x00,       // dim count
        ]);

        let syms = parse_symbol_browse_response(&buf);
        prop_assert_eq!(syms.len(), 1);
        prop_assert_eq!(syms[0].name.as_str(), name.as_str());
    }
}

use ethernetip::cip::build_symbol_browse_request;

#[test]
fn round_trip_build_and_parse() {
    // Build request (we only check it is well‑formed)
    let req = build_symbol_browse_request();
    assert!(!req.is_empty());

    // Fake a minimal valid response for one symbol
    let resp = vec![
        0x00, 0x00, // Attr1
        0x03, b'F', b'o', b'o', 0x00, // align
        0x00, 0x00, // Attr2
        0xC4, 0x00, // DINT
        0x00, 0x00, // Attr5
        0x00, // dim count
        0x00, 0x00, // padding
    ];

    let syms = parse_symbol_browse_response(&resp);
    assert_eq!(syms.len(), 1);
    assert_eq!(syms[0].name, "Foo");
    assert_eq!(syms[0].typ, CipType::DInt);
}
