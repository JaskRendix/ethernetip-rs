# ethernetip‑rs

A Rust implementation of the EtherNet/IP™ protocol for reading and writing CIP tags on Allen‑Bradley ControlLogix and CompactLogix PLCs.  
The library provides a simple async API for symbolic tag access, array operations, and multi‑tag requests, with correct EPATH encoding and slot routing.

---

## Overview

`ethernetip-rs` implements the unconnected CIP messaging path used by Rockwell Logix controllers.  
It supports both CompactLogix (no slot routing) and ControlLogix (CPU located in a chassis slot).

### Supported features

- Read a single tag  
- Write a single tag  
- Write arrays and array fragments  
- Multiple Service Packet (MSP) read/write  
- Correct CIP EPATH encoding (symbolic segments, array indices, slot routing)  
- Optional fake PLC for local testing  
- Async API using `tokio`

### Slot routing

ControlLogix systems require routing through the chassis:

- CompactLogix: CPU is the Ethernet endpoint → **no slot routing**
- ControlLogix: CPU resides in a slot → **slot must be encoded in the EPATH**
- If the Ethernet module is not in slot 1, the API allows specifying both Ethernet‑slot and CPU‑slot

Slot routing is now applied consistently across all read/write operations.

---

## Installation

```toml
[dependencies]
ethernetip = "0.x"
```

---

## Basic usage

```rust
use anyhow::Result;
use ethernetip::EthernetIpClient;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = EthernetIpClient::connect("192.168.1.10", 44818).await?;

    // For ControlLogix racks: route to CPU slot
    client.set_slot(0);

    let value = client.read_tag("MyTag").await?;
    println!("Value: {:?}", value);

    Ok(())
}
```

---

## Reading with explicit CPU slot

```rust
client.set_slot(2); // CPU in slot 2
let v = client.read_tag("SomeTag").await?;
```

---

## Writing a tag

```rust
use ethernetip::types::CipValue;

client.write_tag("MyTag", CipValue::DInt(42)).await?;
```

---

## Writing an array

```rust
use ethernetip::types::CipValue;

let data = CipValue::DIntArray(vec![1, 2, 3, 4]);
client.write_array_tag("MyArray", data).await?;
```

---

## Multiple tag read

```rust
let results = client.read_tags_multi(&["A", "B", "C"]).await?;
```

---

## Fake PLC for testing

A minimal fake PLC is included for local development.  
It implements a subset of CIP services and returns deterministic values.

```rust
tokio::spawn(async {
    ethernetip::fake_plc::run_fake_plc().await.unwrap();
});
```

> Note: the fake PLC uses a simplified EPATH parser and does **not** fully emulate ControlLogix routing behavior.

---

## Running

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cargo run
```

---

## Future improvements (non‑committal)

- Forward Open / Forward Close (connected messaging)
- Class/instance/attribute access for non‑Logix devices
- Expanded fake PLC with more CIP types and error codes
- Better timeout/retry handling
- Benchmarks for read/write/MSP performance
- Optional implicit I/O (UDP) support
- More examples and protocol documentation
- Validation against real ControlLogix/CompactLogix hardware

---

## Notes

This Rust port was originally written for a job interview.  
Real hardware testing would further validate correctness beyond the fake PLC and unit tests.
