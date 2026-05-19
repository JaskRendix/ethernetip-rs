use ethernetip::cip::CipError;
use ethernetip::types::CipValue;

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

#[test]
fn type_mismatch_error_formats_correctly() {
    let err = CipError::TypeMismatch {
        expected: "DINT",
        actual: "REAL",
    };

    let msg = format!("{err}");
    assert_eq!(msg, "Type mismatch: expected DINT, got REAL");
}

#[test]
fn cip_error_display_variants() {
    assert_eq!(
        format!("{}", CipError::ConnectionFailure),
        "Connection failure (0x01)"
    );
    assert_eq!(
        format!("{}", CipError::ResourceUnavailable),
        "Resource unavailable (0x02)"
    );
    assert_eq!(
        format!("{}", CipError::InvalidAttribute),
        "Invalid attribute (0x04)"
    );
    assert_eq!(
        format!("{}", CipError::PathSegmentError),
        "Path segment error (0x05)"
    );
    assert_eq!(
        format!("{}", CipError::PathDestinationUnknown),
        "Path destination unknown (0x06)"
    );
    assert_eq!(
        format!("{}", CipError::VendorSpecific(0xFF)),
        "Vendor-specific CIP error 0xFF"
    );
}

#[test]
fn cip_value_type_name() {
    assert_eq!(CipValue::Bool(true).type_name(), "BOOL");
    assert_eq!(CipValue::SInt(1).type_name(), "SINT");
    assert_eq!(CipValue::Int(1).type_name(), "INT");
    assert_eq!(CipValue::DInt(1).type_name(), "DINT");
    assert_eq!(CipValue::LInt(1).type_name(), "LINT");
    assert_eq!(CipValue::Real(1.0).type_name(), "REAL");
    assert_eq!(CipValue::String("x".into()).type_name(), "STRING");
    assert_eq!(CipValue::BoolPacked(vec![1]).type_name(), "BOOL_PACKED");
    assert_eq!(CipValue::Unit.type_name(), "UNIT");
}
