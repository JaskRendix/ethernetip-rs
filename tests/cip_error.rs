use ethernetip::cip::{map_extended_status, CipError, ForwardOpenError};

// --- CipError::From<u8> ---

#[test]
fn test_cip_error_from_known_codes() {
    assert_eq!(CipError::from(0x01), CipError::ConnectionFailure);
    assert_eq!(CipError::from(0x02), CipError::ResourceUnavailable);
    assert_eq!(CipError::from(0x04), CipError::InvalidAttribute);
    assert_eq!(CipError::from(0x05), CipError::PathSegmentError);
    assert_eq!(CipError::from(0x06), CipError::PathDestinationUnknown);
}

#[test]
fn test_cip_error_from_unknown_code_is_vendor_specific() {
    // 0x03 and 0x07+ aren't mapped explicitly — must fall through, not panic
    assert_eq!(CipError::from(0x03), CipError::VendorSpecific(0x03));
    assert_eq!(CipError::from(0xEE), CipError::VendorSpecific(0xEE));
}

// --- CipError::From<io::Error> ---

#[test]
fn test_cip_error_from_io_error_preserves_message() {
    let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "connection timed out");
    let cip_err: CipError = io_err.into();
    match cip_err {
        CipError::Io(msg) => assert!(msg.contains("timed out")),
        other => panic!("expected CipError::Io, got {other:?}"),
    }
}

// --- CipError::General ---

#[test]
fn test_cip_error_general_with_extended_status() {
    let err = CipError::General {
        status: 0x01,
        extended: vec![0x0205, 0x0315],
    };
    let msg = err.to_string();
    assert!(msg.contains("0x01"));
    assert!(msg.contains("517")); // Debug fmt of u16 vec — see note below
}

#[test]
fn test_cip_error_general_without_extended_status_still_displays() {
    let err = CipError::General {
        status: 0x08,
        extended: vec![],
    };
    let msg = err.to_string();
    assert!(msg.contains("0x08"));
    assert!(!msg.contains("extended")); // empty branch shouldn't mention it
}

// --- Display never panics, always non-empty (cheap but catches typos/fmt bugs) ---

#[test]
fn test_all_cip_error_variants_display_nonempty() {
    let variants = vec![
        CipError::ConnectionFailure,
        CipError::ResourceUnavailable,
        CipError::InvalidAttribute,
        CipError::PathSegmentError,
        CipError::PathDestinationUnknown,
        CipError::VendorSpecific(0x42),
        CipError::TypeMismatch {
            expected: "DINT",
            actual: "REAL",
        },
        CipError::General {
            status: 0x01,
            extended: vec![0x0100],
        },
        CipError::Io("simulated failure".to_string()),
    ];
    for v in variants {
        assert!(!v.to_string().is_empty());
    }
}

// --- Clone/PartialEq sanity (regression guard: these derives are load-bearing now) ---

#[test]
fn test_cip_error_clone_eq() {
    let a = CipError::General {
        status: 0x01,
        extended: vec![1, 2, 3],
    };
    let b = a.clone();
    assert_eq!(a, b);
}

// --- map_extended_status ---

#[test]
fn test_map_extended_status_empty() {
    match map_extended_status(&[]) {
        ForwardOpenError::ExtendedStatus(words) => assert!(words.is_empty()),
        other => panic!("expected ExtendedStatus([]), got {other:?}"),
    }
}

#[test]
fn test_map_extended_status_known_codes() {
    assert!(matches!(
        map_extended_status(&[0x0100]),
        ForwardOpenError::Timeout
    ));
    assert!(matches!(
        map_extended_status(&[0x0204]),
        ForwardOpenError::InvalidSize
    ));
    assert!(matches!(
        map_extended_status(&[0x0205]),
        ForwardOpenError::InvalidRpi
    ));
    assert!(matches!(
        map_extended_status(&[0x0315]),
        ForwardOpenError::ResourceUnavailable
    ));
    assert!(matches!(
        map_extended_status(&[0x0316]),
        ForwardOpenError::UnsupportedTrigger
    ));
}

#[test]
fn test_map_extended_status_unknown_code_falls_through() {
    match map_extended_status(&[0x9999]) {
        ForwardOpenError::ExtendedStatus(words) => assert_eq!(words, vec![0x9999]),
        other => panic!("expected ExtendedStatus([0x9999]), got {other:?}"),
    }
}

#[test]
fn test_map_extended_status_only_reads_first_word() {
    // Confirms documented behavior: only words[0] is inspected, extras ignored.
    // If this ever changes to fold in later words, this test should change too —
    // that's the point of having it pinned.
    match map_extended_status(&[0x0205, 0xFFFF]) {
        ForwardOpenError::InvalidRpi => {}
        other => panic!("expected InvalidRpi, got {other:?}"),
    }
}

// --- ForwardOpenError::From<io::Error> ---

#[test]
fn test_forward_open_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let fo_err: ForwardOpenError = io_err.into();
    match fo_err {
        ForwardOpenError::Other(msg) => assert!(msg.contains("refused")),
        other => panic!("expected Other, got {other:?}"),
    }
}
