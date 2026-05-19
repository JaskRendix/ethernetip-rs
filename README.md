# **ethernetip‑rs**

A Rust implementation of the EtherNet/IP™ protocol for symbolic tag access on Allen‑Bradley ControlLogix and CompactLogix PLCs.  
The library provides an async API for reading and writing CIP tags, including typed accessors, arrays, fragmented reads, and multi‑tag operations, with correct EPATH encoding and optional slot routing.

---

## Overview

`ethernetip-rs` implements both unconnected and connected CIP explicit messaging for
Rockwell Logix controllers. The client supports standard SendRRData requests as well
as Class‑3 connected messaging via Forward Open / Forward Close and SendUnitData.  
It supports CompactLogix (no routing) and ControlLogix (CPU in a chassis slot).  
A deterministic fake PLC is included for development and testing.

### Features

- Typed tag access:
  - `read_bool`, `read_sint`, `read_int`, `read_dint`, `read_real`, `read_string`
  - `write_bool`, `write_sint`, `write_int`, `write_dint`, `write_real`, `write_string`
- Raw tag access (`read_tag`, `write_tag`)
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
  - Large Forward Open (0x5B) / Large Forward Close (0x5E)
  - SendUnitData transport
  - connection ID + sequence counter tracking
  - automatic routing of CIP requests over RR‑Data or Unit‑Data

---

## Supported CIP types

- BOOL (including packed BOOL arrays)
- SINT
- INT
- DINT
- LINT
- REAL
- STRING

Typed helpers automatically validate the returned CIP type and return  
`CipError::TypeMismatch { expected, actual }` when the PLC tag type does not match.

---

## Slot routing

ControlLogix systems require routing through the backplane:

- CompactLogix: CPU is the Ethernet endpoint  
- ControlLogix: CPU resides in a slot  

```rust
client.set_slot(2); // CPU in slot 2
```

Routing is applied across all read and write operations.

---

## Basic usage (typed API)

```rust
use ethernetip::EthernetIpClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = EthernetIpClient::connect("192.168.1.10").await?;
    client.set_slot(0);

    let running = client.read_bool("MotorRunning").await?;
    let speed   = client.read_real("MotorSpeed").await?;
    let count   = client.read_dint("Counter").await?;

    client.write_dint("Setpoint", 120).await?;
    client.write_bool("Enable", true).await?;

    Ok(())
}
```

### Raw API (still available)

```rust
use ethernetip::types::CipValue;

let value = client.read_tag("MyTag").await?;
client.write_tag("MyTag", CipValue::DInt(42)).await?;
```

---

## Reading arrays

### Small arrays (single packet)

```rust
let values = client.read_tag_multi("MyArray", 10).await?;
```

### Large arrays (fragmented read)

```rust
let values = client.read_array("LargeArray", 2000).await?;
```

Handles:

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

The test suite includes full coverage for CIP request builders, including Forward Open, Large Forward Open, and all EPATH variants.

---

## Running

```
cargo run
```

---

## Future improvements

- Additional connection types (redundant owner)
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
