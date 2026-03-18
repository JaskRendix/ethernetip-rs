use anyhow::Result;
use ethernetip::{client::EthernetIpClient, fake_plc::run_fake_plc};

#[tokio::main]
async fn main() -> Result<()> {
    tokio::spawn(async {
        run_fake_plc().await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let mut client = EthernetIpClient::connect("127.0.0.1", 44818).await?;
    println!("Connected!");

    let v = client.read_tag("Test").await?;
    println!("Read Test = {:?}", v);

    Ok(())
}
