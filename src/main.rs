use anyhow::Result;
use ethernetip::{client::EthernetIpClient, fake_plc::run_fake_plc};

#[tokio::main]
async fn main() -> Result<()> {
    // Start the fake PLC in the background
    tokio::spawn(async {
        run_fake_plc().await.unwrap();
    });

    // Give it time to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Connect to the simulator
    let mut client = EthernetIpClient::connect("127.0.0.1").await?;
    println!("Connected!");

    // Read a tag
    let v = client.read_tag("Test").await?;
    println!("Read Test = {:?}", v);

    Ok(())
}
