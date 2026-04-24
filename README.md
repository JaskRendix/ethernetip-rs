# **ethernetip‑rs**

A Rust implementation of the EtherNet/IP™ protocol for symbolic tag access on Allen‑Bradley ControlLogix and CompactLogix PLCs.  
The library provides an async API for reading and writing CIP tags, including arrays, fragmented reads, and multi‑tag operations, with correct EPATH encoding and optional slot routing.

---

## Overview

`ethernetip-rs` implements both unconnected and connected CIP explicit messaging for
Rockwell Logix controllers. The client supports standard SendRRData requests as well
as Class‑3 connected messaging via Forward Open / Forward Close and SendUnitData.  
It supports CompactLogix (no routing) and ControlLogix (CPU in a chassis slot).  
A deterministic fake PLC is included for development and testing.

### Features

- Read a single tag  
- Write a single tag  
- Read arrays  
  - unfragmented reads for small arrays  
  - CIP Fragmented Read (0x52) for large arrays  
- Write arrays  
- Multiple Service Packet (MSP) multi‑tag read  
- Correct CIP EPATH encoding  
  - symbolic segments  
  - array indices  
  - multi‑index  
  - struct members  
  - slot routing  
- Async API using `tokio`  
- Fake PLC for integration tests  
- Deterministic behavior for CI environments  
- Connected explicit messaging (Class 3)
  - Forward Open / Forward Close
  - SendUnitData transport
  - connection ID + sequence counter tracking
  - automatic routing of CIP requests over RR‑Data or Unit‑Data  

### Supported CIP types

- BOOL (including packed BOOL arrays)
- SINT
- INT
- DINT
- LINT
- REAL
- STRING

---

## Slot routing

ControlLogix systems require routing through the backplane:

- CompactLogix: CPU is the Ethernet endpoint  
- ControlLogix: CPU resides in a slot  

Example:

```rust
client.set_slot(2); // CPU in slot 2
```

Routing is applied across all read and write operations.

---

## Basic usage

```rust
use ethernetip::EthernetIpClient;
use ethernetip::types::CipValue;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = EthernetIpClient::connect("192.168.1.10").await?;
    client.set_slot(0);

    let value = client.read_tag("MyTag").await?;
    println!("Value: {:?}", value);

    client.write_tag("MyTag", CipValue::DInt(42)).await?;
    Ok(())
}
```

---

## Reading arrays

### Small arrays (single packet)

```rust
let values = client.read_tag_multi("MyArray", 10).await?;
```

### Large arrays (fragmented read)

Logix controllers limit unfragmented reads to ~480 bytes.  
`read_array()` performs CIP Fragmented Read (0x52) and reconstructs the full payload.

```rust
let values = client.read_array("LargeArray", 2000).await?;
```

The client handles:

- type ID extraction  
- partial transfer status (0x06)  
- offset increments  
- fragment concatenation  
- decoding into `Vec<CipValue>`  

---

## Writing arrays

```rust
use ethernetip::types::CipValue;

let data = vec![
    CipValue::DInt(1),
    CipValue::DInt(2),
    CipValue::DInt(3),
];

client.write_tag_multi("MyArray", &data).await?;
```

---

## Multiple tag read (MSP)

```rust
let results = client.read_tags_msp(&["A", "B", "C"]).await?;

for r in results {
    match r {
        MultiResult::Ok(v) => println!("Value: {:?}", v),
        MultiResult::Err(code) => println!("Error: 0x{:02X}", code),
    }
}
```

MSP batches multiple CIP requests into one round‑trip.

---

## Fake PLC for testing

The fake PLC supports:

- read (single and array)  
- write (single and array)  
- MSP  
- symbol browse  
- error simulation (`FAKE_PLC_ERROR=1`)  

Example:

```rust
tokio::spawn(async {
    ethernetip::fake_plc::run_fake_plc().await.unwrap();
});
```

This enables end‑to‑end tests without hardware.

---

## Running

```
cargo run
```

---

## Future improvements

- Additional connection types (large forward open, redundant owner)
- Automatic reconnect for connected sessions
- Implicit I/O (UDP)
- Class/instance/attribute access for non‑Logix devices  
- More realistic fake PLC behavior  
- Retry and backoff logic  
- Benchmarks for MSP and array operations  

---

## Notes

This project began as a technical exercise and grew into a functional EtherNet/IP implementation.  
Hardware testing is recommended for production use.  
The fake PLC and test suite provide a baseline for development and CI.
