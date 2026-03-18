use ethernetip::fake_plc::run_fake_plc;
use ethernetip::types::CipValue;
use ethernetip::EthernetIpClient;

#[tokio::test]
async fn read_bool_from_fake_plc() {
    // Start fake PLC in background
    tokio::spawn(async {
        let _ = run_fake_plc().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1", 44818)
        .await
        .expect("connect failed");

    let value = client.read_tag("Test").await.unwrap();
    assert_eq!(value, Some(CipValue::Bool(true)));
}
