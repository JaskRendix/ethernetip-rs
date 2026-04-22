use ethernetip::fake_plc::run_fake_plc;
use ethernetip::types::{CipValue, MultiResult};
use ethernetip::EthernetIpClient;
use std::sync::Once;
use std::time::Duration;
use tokio::time::sleep;

static START: Once = Once::new();

fn start_fake_plc_once() {
    START.call_once(|| {
        tokio::spawn(async {
            let _ = run_fake_plc().await;
        });
    });
}

//
// ─────────────────────────────────────────────────────────────
//   Multi‑element READ (fake PLC returns 42, 43, 44…)
// ─────────────────────────────────────────────────────────────
//

#[tokio::test]
async fn read_multi_elements_from_fake_plc() {
    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    // Read 3 elements: Test[0], Test[1], Test[2]
    let vals = client.read_tag_multi("Test", 3).await.unwrap();

    assert_eq!(vals.len(), 3);
    assert_eq!(vals[0], CipValue::DInt(42));
    assert_eq!(vals[1], CipValue::DInt(43));
    assert_eq!(vals[2], CipValue::DInt(44));
}

//
// ─────────────────────────────────────────────────────────────
//   Multi‑element WRITE (fake PLC accepts all writes)
// ─────────────────────────────────────────────────────────────
//

#[tokio::test]
async fn write_multi_elements_to_fake_plc() {
    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    let values = vec![CipValue::DInt(10), CipValue::DInt(20), CipValue::DInt(30)];

    let res = client.write_tag_multi("Test", &values).await;
    assert!(res.is_ok());
}

//
// ─────────────────────────────────────────────────────────────
//   MSP multi‑read (client → fake PLC → MSP → client)
// ─────────────────────────────────────────────────────────────
//

#[tokio::test]
async fn msp_multi_read_from_fake_plc() {
    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    let tags = vec!["Test", "Test", "Test"];
    let results = client.read_tags_msp(&tags).await.unwrap();

    assert_eq!(results.len(), 3);

    for r in results {
        match r {
            MultiResult::Ok(CipValue::DInt(v)) => assert_eq!(v, 42),
            _ => panic!("unexpected MSP result"),
        }
    }
}

//
// ─────────────────────────────────────────────────────────────
//   MSP error simulation (FAKE_PLC_ERROR=1)
// ─────────────────────────────────────────────────────────────
//

#[tokio::test]
async fn msp_error_simulation_every_5th_request() {
    std::env::set_var("FAKE_PLC_ERROR", "1");

    start_fake_plc_once();
    sleep(Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1")
        .await
        .expect("connect failed");

    let tags = vec!["Test", "Test", "Test", "Test", "Test"];

    let results = client.read_tags_msp(&tags).await.unwrap();

    // At least one should be an error (every 5th request)
    assert!(results.iter().any(|r| matches!(r, MultiResult::Err(_))));
}
