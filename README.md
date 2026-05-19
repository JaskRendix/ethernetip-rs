# **ethernetip‑rs**

A Rust implementation of the EtherNet/IP™ protocol for symbolic tag access on Allen‑Bradley ControlLogix and CompactLogix PLCs.  
The library provides an async API for reading and writing CIP tags, including typed accessors, arrays, fragmented reads, multi‑tag operations, EPATH encoding, slot routing, and generic CIP object access.

---

## Overview

`ethernetip-rs` implements unconnected and connected CIP explicit messaging for Rockwell Logix controllers.  
The client supports SendRRData and Class‑3 connected messaging through Forward Open / Forward Close and SendUnitData.  
It works with CompactLogix (no routing) and ControlLogix (CPU in a chassis slot).  
A deterministic fake PLC is included for development and CI.

In addition to symbolic tag access, the library supports **generic CIP object reads**, including:

- Identity Object (class 0x01)  
- Connection Manager (class 0x06)  
- Any class/instance/attribute via GetAttributeSingle / GetAttributeAll  

This enables diagnostics and metadata retrieval from both Logix and non‑Logix CIP devices.

---

## Features

### Tag access
- Typed tag access:
  - `read_bool`, `read_sint`, `read_int`, `read_dint`, `read_real`, `read_string`
  - `write_bool`, `write_sint`, `write_int`, `write_dint`, `write_real`, `write_string`
- Raw tag access (`read_tag`, `write_tag`)
- Array reads:
  - single‑packet reads  
  - CIP Fragmented Read (0x52) for large arrays
- Array writes
- Multiple Service Packet (MSP) multi‑tag read

### CIP object access
- Generic object attribute reads:
  - `read_object_attribute(class, instance, attribute)`
  - `read_object_attributes(class, instance)`
- Identity Object support:
  - `read_identity()`
  - `IdentityInfo` decoding (vendor ID, product code, revision, serial, product name)
- Connection Manager diagnostics:
  - `read_connection_manager()`
  - `ConnectionManagerInfo` decoding (state, watchdog timeout, transport class, etc.)

### Protocol correctness
- Full CIP EPATH encoding:
  - symbolic segments  
  - array indices  
  - multi‑index  
  - struct members  
  - slot routing  
- Async API using `tokio`
- Connected explicit messaging (Class 3):
  - Forward Open / Forward Close  
  - Large Forward Open (0x5B) / Large Forward Close (0x5E)  
  - SendUnitData transport  
  - connection ID and sequence counter tracking  
  - automatic routing over RR‑Data or Unit‑Data  

### Testing
- Fake PLC for integration tests  
- Deterministic behavior for CI  
- Unit tests for:
  - CIP builders  
  - EPATH encoding  
  - IdentityInfo decoding  
  - ConnectionManagerInfo decoding  
  - default trait implementations for object access  

---

## Supported CIP types

- BOOL (including packed BOOL arrays)  
- SINT  
- INT  
- DINT  
- LINT  
- REAL  
- STRING  

Typed helpers validate the returned CIP type and return  
`CipError::TypeMismatch { expected, actual }` when the PLC tag type differs.

---

## Slot routing

ControlLogix systems require routing through the backplane:

- CompactLogix: CPU is the Ethernet endpoint  
- ControlLogix: CPU resides in a slot  

```rust
client.set_slot(2);
```

Routing applies to all read and write operations.

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

---

## Raw API

```rust
use ethernetip::types::CipValue;

let value = client.read_tag("MyTag").await?;
client.write_tag("MyTag", CipValue::DInt(42)).await?;
```

---

## Reading arrays

### Small arrays

```rust
let values = client.read_tag_multi("MyArray", 10).await?;
```

### Large arrays (fragmented)

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

MSP batches multiple CIP requests into one round trip.

---

## CIP object access examples

### Identity Object

```rust
let info = client.read_identity().await?;
println!("Product: {} v{}.{}", info.product_name, info.major, info.minor);
```

### Connection Manager

```rust
let cm = client.read_connection_manager().await?;
println!("Connection state: {:?}", cm.state);
```

---

## Fake PLC for testing

The fake PLC simulates:

- tag reads and writes  
- fragmented reads  
- MSP  
- Forward Open / Forward Close  
- EPATH parsing  

Used extensively in CI to ensure deterministic behavior.

---

## Running

```
cargo run
```

---

## Future improvements

- Additional connection types  
- Automatic reconnect for connected sessions  
- Implicit I/O (UDP)  
- More CIP object models  
- More realistic fake PLC behavior  
- Retry and backoff logic  
- Benchmarks for MSP and array operations  

---

## Notes

This project began as a technical exercise and grew into a functional EtherNet/IP implementation.  
Hardware testing is recommended for production use.  
The fake PLC and test suite provide a baseline for development and CI.
