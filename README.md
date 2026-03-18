# ethernetip-rs

This project is a Rust re‑implementation of the original EthernetIP4J library. It provides a simple interface for reading and writing tags on ControlLogix and CompactLogix PLCs using CIP over EtherNet/IP.

## Origin

This project is a Rust re‑implementation of the original EthernetIp4j library:

https://code.google.com/archive/p/ethernetip4j/

The original EthernetIp4j project is a communication protocol library for Rockwell Allen‑Bradley PLC systems, written in Java and released under the Apache License 2.0. This Rust version is an independent port that preserves the same licensing terms.

## Original project

The original EthernetIp4j project was written in Java and is available here:

https://github.com/tuliomagalhaes/Ethernetip4j

EthernetIp4j is a communication protocol library for Rockwell Allen‑Bradley PLC systems, implemented entirely in Java. This Rust version is a separate re‑implementation inspired by the original design.

## Overview

The client communicates with a single Ethernet module in a ControlLogix rack or with a CompactLogix CPU.  
For ControlLogix systems, the slot number of the CPU matters. The Ethernet module forwards requests to the CPU slot you specify.

- If the Ethernet module is in slot 1 and the CPU is in slot 0, you can read and write tags without specifying a slot.
- If the CPU is in another slot, set the slot before issuing reads or writes.
- If the Ethernet module itself is not in slot 1, use the API that accepts both Ethernet‑slot and CPU‑slot.

The library supports:

- Reading a single tag
- Writing a single tag
- Writing arrays
- Writing array fragments
- Multiple Service Packet (MSP) read and write
- A built‑in fake PLC for local testing

## Basic usage

```rust
use anyhow::Result;
use ethernetip::EthernetIpClient;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = EthernetIpClient::connect("192.168.1.10", 44818).await?;

    // Optional: set CPU slot for ControlLogix racks
    client.set_slot(0);

    let value = client.read_tag("MyTag").await?;
    println!("Value: {:?}", value);

    Ok(())
}
```

## Reading with explicit CPU slot

```rust
client.set_slot(2);
let v = client.read_tag("SomeTag").await?;
```

## Writing a tag

```rust
use ethernetip::types::CipValue;

client.write_tag("MyTag", CipValue::DInt(42)).await?;
```

## Writing an array

```rust
use ethernetip::types::CipValue;

let data = CipValue::DIntArray(vec![1, 2, 3, 4]);
client.write_array_tag("MyArray", data).await?;
```

## Multiple tag read

```rust
let results = client.read_tags_multi(&["A", "B", "C"]).await?;
```

## Fake PLC for testing

A simple fake PLC is included. It listens on port 44818 and returns a fixed BOOL value for testing.

```rust
tokio::spawn(async {
    ethernetip::fake_plc::run_fake_plc().await.unwrap();
});
```

## Running

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cargo run
```

## Potential ideas

These are possible future improvements. They are not commitments and may or may not be implemented.

- Add Forward Open and Forward Close to support connected messaging.
- Add class/instance/attribute access for devices that do not use symbolic tags.
- Expand the fake PLC to simulate more CIP types, error codes, and multi-tag responses.
- Improve error handling for timeouts, retries, and network interruptions.
- Add benchmarks for read, write, and MSP performance.
- Add support for implicit I/O messaging (UDP) for devices that require it.
- Add documentation for all CIP services currently implemented.
- Add examples for MSP write and fragmented array writes.
- Find access to a real ControlLogix or CompactLogix system to validate behavior against actual hardware.

## Notes

This Rust port was written for a job interview. The interview was not successful = no job.

Real hardware testing would help confirm correctness beyond the current fake PLC and unit tests. Access to a ControlLogix or CompactLogix system would be ideal. PLC‑controlled CNC equipment used in industrial sectors such as stone and marble processing could also serve as a suitable test environment, provided the machines use Allen‑Bradley controllers with EtherNet/IP support.
