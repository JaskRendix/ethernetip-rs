use ethernetip::cip::decode_cip_response;
use ethernetip::CipValue;

#[test]
fn decode_bool_true() {
    let buf = vec![0xC1, 0x00, 0x01];
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Bool(true));
}

#[test]
fn decode_dint() {
    let buf = vec![0xC4, 0x00, 0x2A, 0x00, 0x00, 0x00]; // DINT 42
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::DInt(42));
}

#[test]
fn decode_real() {
    let buf = vec![0xCA, 0x00, 0x00, 0x00, 0x20, 0x41]; // REAL 10.0
    let v = decode_cip_response(&buf).unwrap();
    assert_eq!(v, CipValue::Real(10.0));
}
