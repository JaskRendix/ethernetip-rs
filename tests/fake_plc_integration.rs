use ethernetip::cip::*;
use ethernetip::fake_plc::run_fake_plc;
use ethernetip::types::{CipType, CipValue};
use ethernetip::EthernetIpClient;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;

// Start the fake PLC only once for the entire test suite
static START: Once = Once::new();

fn start_fake_plc_once() {
    START.call_once(|| {
        tokio::spawn(async {
            let _ = run_fake_plc().await;
        });
    });
}

#[tokio::test]
async fn read_from_fake_plc() {
    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    let value = client.read_tag("Test").await.unwrap();
    assert_eq!(value, CipValue::DInt(42));
}

#[tokio::test]
async fn write_to_fake_plc() {
    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    let result = client.write_tag("Test", CipValue::DInt(123)).await;
    assert!(result.is_ok());
}

#[test]
fn cpf_parsing_extracts_cip_payload() {
    let cpf = vec![
        0, 0, 0, 0, 0, 0, // interface + timeout
        0x02, 0x00, // item count = 2
        // Item 1: Null address
        0x00, 0x00, 0x00, 0x00, // Item 2: Data item
        0xB2, 0x00, 0x03, 0x00, 0xC1, 0x00, 0x01, // BOOL TRUE
    ];

    let extracted = EthernetIpClient::parse_cpf(&cpf).unwrap();
    assert_eq!(extracted, &[0xC1, 0x00, 0x01]);
}

#[test]
fn decode_bool_true() {
    let buf = vec![0xC1, 0x00, 0x01];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Bool(true));
}

#[test]
fn decode_dint() {
    let buf = vec![0xC4, 0x00, 0x2A, 0x00, 0x00, 0x00];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::DInt(42));
}

#[test]
fn decode_real() {
    let buf = vec![0xCA, 0x00, 0x00, 0x00, 0x20, 0x41];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Real(10.0));
}

#[test]
fn read_request_encoding() {
    let cip = build_read_request("Test", None);

    assert_eq!(cip[0], 0x4C);
    assert_eq!(cip[1], 3);
    assert_eq!(cip[2], 0x91);
    assert_eq!(cip[3], 4);
    assert_eq!(&cip[4..8], b"Test");
    assert_eq!(&cip[8..10], &[1, 0]);
}

#[test]
fn write_request_encoding() {
    let cip = build_write_request("Tag", &CipValue::DInt(123), None);

    assert_eq!(cip[0], 0x4D);
    assert_eq!(cip[1], 3);
    assert_eq!(cip[2], 0x91);
    assert_eq!(cip[3], 3);
    assert_eq!(&cip[4..7], b"Tag");
    assert_eq!(cip[7], 0x00);
    assert_eq!(&cip[8..10], &(CipType::DInt as u16).to_le_bytes());
    assert_eq!(&cip[10..12], &[1, 0]);
    assert_eq!(&cip[12..16], &123i32.to_le_bytes());
}

#[test]
fn encode_simple_tag() {
    let e = encode_epath("Test");
    assert_eq!(e, vec![0x03, 0x91, 0x04, b'T', b'e', b's', b't']);
}

#[test]
fn encode_odd_length_tag() {
    let e = encode_epath("Abc");
    assert_eq!(e, vec![0x03, 0x91, 0x03, b'A', b'b', b'c', 0x00]);
}

#[test]
fn encode_array_index() {
    let e = encode_epath("Array[5]");
    assert_eq!(
        e,
        vec![0x05, 0x91, 0x05, b'A', b'r', b'r', b'a', b'y', 0x00, 0x28, 0x05]
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
fn encode_struct_member() {
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
fn encode_epath_with_slot_test() {
    let e = encode_epath_with_slot("Tag", Some(3));

    assert_eq!(
        e,
        vec![
            0x05, // 5 words
            0x01, 0x03, 0x00, 0x00, // correct 4‑byte port segment
            0x91, 0x03, b'T', b'a', b'g', 0x00
        ]
    );
}

#[test]
fn map_status_codes() {
    assert!(matches!(CipError::from(0x01), CipError::ConnectionFailure));
    assert!(matches!(CipError::from(0x05), CipError::PathSegmentError));
    assert!(matches!(
        CipError::from(0x06),
        CipError::PathDestinationUnknown
    ));
    assert!(matches!(
        CipError::from(0xAB),
        CipError::VendorSpecific(0xAB)
    ));
}

#[tokio::test]
async fn browse_symbols_from_fake_plc() {
    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    let syms = client.browse_symbols().await.unwrap();
    assert_eq!(syms.len(), 1);

    let s = &syms[0];
    assert_eq!(s.name, "TestTag");
    assert_eq!(s.typ, CipType::DInt);
    assert!(s.array_dims.is_none());
}
