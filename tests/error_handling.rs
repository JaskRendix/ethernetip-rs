use ethernetip::cip::CipError;

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
