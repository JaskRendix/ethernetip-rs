use ethernetip::cip::{
    build_cip_multiple_service_request, parse_cip_multiple_service_response, CipService,
};

#[test]
fn build_and_parse_msp() {
    let req1 = vec![CipService::ReadData as u8, 0x00];
    let req2 = vec![CipService::ReadData as u8, 0x00];

    let _msp = build_cip_multiple_service_request(&[req1.clone(), req2.clone()]);

    // Fake a response: service|0x80, path=0, count=2, offsets...
    let response = vec![
        0x8A, 0x00, 0x02, 0x00, 0x08, 0x00, 0x0A, 0x00, 0xCC, 0x00, 0xC1, 0x00, 0x01, 0xCC, 0x00,
        0xC1, 0x00, 0x00,
    ];

    let results = parse_cip_multiple_service_response(&response);
    assert_eq!(results.len(), 2);
}
