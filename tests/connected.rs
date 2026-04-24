use ethernetip::{types::CipValue, EthernetIpClient};

#[tokio::test]
async fn test_forward_open_and_close() {
    tokio::spawn(async {
        ethernetip::fake_plc::run_fake_plc().await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1").await.unwrap();
    client.set_slot(0);

    client.forward_open().await.unwrap();
    assert!(client.is_connected());

    client.forward_close().await.unwrap();
    assert!(!client.is_connected());
}

#[tokio::test]
async fn test_connected_read_write() {
    tokio::spawn(async {
        ethernetip::fake_plc::run_fake_plc().await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1").await.unwrap();
    client.set_slot(0);

    client.forward_open().await.unwrap();

    client
        .write_tag("DINTTag", CipValue::DInt(123))
        .await
        .unwrap();
    let v = client.read_tag("DINTTag").await.unwrap();

    assert_eq!(v, CipValue::DInt(42)); // fake PLC always returns 42 + index
    client.forward_close().await.unwrap();
}

#[tokio::test]
async fn test_unit_data_without_connection_fails() {
    tokio::spawn(async {
        ethernetip::fake_plc::run_fake_plc().await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1").await.unwrap();

    let cip = ethernetip::cip::build_read_request("DINTTag", None);
    let err = client.try_send_unit_data(cip).await.unwrap_err();

    assert!(err.to_string().contains("No active ForwardOpen connection"));
}

#[tokio::test]
async fn test_sequence_counter_increments() {
    tokio::spawn(async {
        ethernetip::fake_plc::run_fake_plc().await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1").await.unwrap();
    client.set_slot(0);

    client.forward_open().await.unwrap();
    let initial = client.sequence();

    let _ = client.read_tag("DINTTag").await.unwrap();
    let after_one = client.sequence();

    let _ = client.read_tag("DINTTag").await.unwrap();
    let after_two = client.sequence();

    assert_eq!(after_one, initial + 1);
    assert_eq!(after_two, initial + 2);

    client.forward_close().await.unwrap();
}
