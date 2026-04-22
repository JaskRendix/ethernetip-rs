# ethernetip‑rs

A Rust implementation of the EtherNet/IP™ protocol for symbolic tag access on Allen‑Bradley ControlLogix and CompactLogix PLCs.  
The library provides a clean async API for reading and writing CIP tags, including arrays and multi‑tag operations, with correct EPATH encoding and optional slot routing.

---

## Overview

`ethernetip-rs` implements the unconnected CIP messaging path used by Rockwell Logix controllers.  
It supports both CompactLogix (no routing) and ControlLogix (CPU in a chassis slot), and includes a fully deterministic fake PLC for local development and testing.

### Features

- Read a single tag  
- Write a single tag  
- Read multiple elements (array fragments)  
- Write multiple elements  
- Multiple Service Packet (MSP) multi‑tag read  
- Correct CIP EPATH encoding  
  - symbolic segments  
  - array indices  
  - multi‑index  
  - struct members  
  - slot routing  
- Async API using `tokio`  
- Optional fake PLC for integration tests  
- Deterministic behavior for CI environments  

---

## Slot routing

ControlLogix systems require routing through the backplane:

- CompactLogix: CPU is the Ethernet endpoint → no routing  
- ControlLogix: CPU resides in a slot → slot must be encoded in the EPATH  

Example:

```rust
client.set_slot(2); // CPU in slot 2
```

Slot routing is applied consistently across all read/write operations.

---

## Basic usage

```rust
use ethernetip::EthernetIpClient;
use ethernetip::types::CipValue;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut client = EthernetIpClient::connect("192.168.1.10").await?;

    // For ControlLogix racks
    client.set_slot(0);

    let value = client.read_tag("MyTag").await?;
    println!("Value: {:?}", value);

    client.write_tag("MyTag", CipValue::DInt(42)).await?;

    Ok(())
}
```

---

## Reading arrays

```rust
let values = client.read_tag_multi("MyArray", 10).await?;
println!("First 10 elements: {:?}", values);
```

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

MSP allows batching multiple CIP requests into a single round‑trip.

---

## Fake PLC for testing

A deterministic fake PLC is included for local development and CI.  
It implements:

- Read (single + array)  
- Write (single + array)  
- MSP  
- Symbol browse  
- Error simulation (every 5th request when `FAKE_PLC_ERROR=1`)  

Example:

```rust
tokio::spawn(async {
    ethernetip::fake_plc::run_fake_plc().await.unwrap();
});
```

This allows full end‑to‑end testing without hardware.

---

## Running

```
cargo run
```

---

## Future improvements

These are non‑committal ideas for future expansion:

- Forward Open / Forward Close (connected messaging)  
- Class/instance/attribute access for non‑Logix devices  
- Additional CIP types (SINT, REAL arrays, STRING, structures)  
- More realistic fake PLC behavior  
- Retry/backoff logic  
- Benchmarks for MSP and array operations  
- Optional implicit I/O (UDP) support  

---

## Notes

This project originated as a technical exercise and evolved into a functional EtherNet/IP implementation.  
Real hardware testing is recommended for production use, but the fake PLC and test suite provide strong baseline validation.
