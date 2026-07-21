# RkyvClone Macro - Automatic rkyv Struct Generation

## Overview

The `RkyvClone` derive macro automatically generates rkyv-optimised parallel structs for
high-performance serialization/deserialization.

## Performance Improvements

**AlertData benchmark results:**

- **Serialize**: 258ns (JSON) → 122ns (rkyv) = **2.1x faster**
- **Deserialize**: 306ns (JSON) → 82ns (rkyv) = **3.7x faster**

**FusedData (from existing benchmarks):**

- **Deserialize**: 3,200ns (JSON) → 305ns (rkyv) = **10.5x faster**

## How It Works

The macro:

1. Creates a parallel struct with `R` prefix (e.g., `AlertData` → `RAlertData`)
2. Converts `DateTime<Utc>` fields to `i64` (timestamp_millis)
3. Recursively converts nested custom types (e.g., `System` → `RSystem`)
4. Generates bidirectional `From` implementations
5. Adds rkyv derive macros (`Archive`, `Serialize`, `Deserialize`)

## Usage

### Basic Example

```rust
use fetiche_macros::RkyvClone;
use chrono::{DateTime, Utc};

#[cfg_attr(feature = "rkyv", derive(RkyvClone))]
#[derive(Debug, Deserialize, Serialize)]
pub struct AlertData {
    pub version: Option<String>,
    pub title: String,
    pub timestamp: DateTime<Utc>,  // Auto-converted to timestamp_millis: i64
    pub severity: Severity,         // Enum kept as-is (must impl rkyv traits)
    pub details: String,
}
```

**Generated code (conceptual):**

```rust
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq)]
pub struct RAlertData {
    pub version: Option<String>,
    pub title: String,
    pub timestamp_millis: i64,  // DateTime → i64
    pub severity: Severity,
    pub details: String,
}

impl From<&AlertData> for RAlertData { /* conversion */ }
impl From<&RAlertData> for AlertData { /* conversion */ }
```

### Using the Generated Struct

```rust
// Original struct
let alert = AlertData {
timestamp: Utc::now(),
title: "System Alert".to_string(),
// ...
};

// Convert to rkyv version (zero-copy ready)
let rkyv_alert: RAlertData = ( & alert).into();

// Serialize with rkyv
let bytes = rkyv::to_bytes::<rkyv::rancor::Error>( & rkyv_alert) ?;

// Deserialize (3-10x faster than serde_json)
let decoded = rkyv::from_bytes::<RAlertData, rkyv::rancor::Error>( & bytes) ?;

// Convert back if needed
let recovered: AlertData = ( & decoded).into();
```

## Type Conversions

### Automatic Conversions

| Original Type   | rkyv Type     | Conversion Logic                                               |
|-----------------|---------------|----------------------------------------------------------------|
| `DateTime<Utc>` | `i64`         | `.timestamp_millis()` / `from_timestamp_millis()`              |
| `CustomData`    | `RCustomData` | Recursive `From` impl (if type ends in Data/State/System/etc.) |
| `Vec<T>`        | `Vec<RT>`     | Element-wise conversion if T is custom                         |
| `Option<T>`     | `Option<RT>`  | Element-wise conversion if T is custom                         |
| Primitives      | Same          | Direct clone                                                   |

### Custom Types Recognized

The macro converts types ending with:

- `Data` (e.g., `FusedData` → `RFusedData`)
- `State` (e.g., `VehicleState` → `RVehicleState`)
- `System` (e.g., `System` → `RSystem`)
- `Identification`, `Value`, `Altitudes`, `Location`, `Coordinates`, `Log`

Enums and other types are used as-is (they must implement rkyv traits).

## Requirements

### For Enums Used in Structs

Enums must derive Clone, PartialEq, and rkyv traits:

```rust
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}
```

### Cargo.toml Setup

```toml
[dependencies]
fetiche-macros = { path = "../macros", version = "0.4.0" }
rkyv = { version = "0.8", optional = true }

[features]
rkyv = ["dep:rkyv"]
```

## Advanced: Nested Structs

The macro handles nested structures automatically:

```rust
#[cfg_attr(feature = "rkyv", derive(RkyvClone))]
#[derive(Debug, Deserialize, Serialize)]
pub struct FusedData {
    pub system: System,           // → RSystem
    pub vehicle_state: VehicleState,  // → RVehicleState
    pub timestamp: DateTime<Utc>,    // → timestamp_millis: i64
}

#[cfg_attr(feature = "rkyv", derive(RkyvClone))]
#[derive(Debug, Deserialize, Serialize)]
pub struct System {
    pub track_id: String,
    pub timestamp: DateTime<Utc>,
}
```

Generated conversions cascade automatically.

## Limitations

1. **Enums**: Must manually add rkyv derives (macro doesn't auto-generate R-prefixed enums)
2. **Custom conversion logic**: Not supported (e.g., can't customize DateTime conversion)
3. **Type detection**: Only converts types ending with specific suffixes
4. **serde attributes**: Not transferred to rkyv structs (e.g., `#[serde(rename)]`)

## When to Use

✅ **Use rkyv when:**

- High-frequency serialization (>1000 ops/sec)
- Performance-critical paths (API endpoints, message queues)
- Large data volumes (batching, streaming)
- Zero-copy deserialization beneficial

❌ **Stay with serde when:**

- Human-readable output needed (debugging, logging)
- Interoperability with external systems (JSON APIs)
- Schema evolution/versioning critical
- One-time or low-frequency operations

## Next Steps

To add rkyv to more types:

1. Add `#[cfg_attr(feature = "rkyv", derive(RkyvClone))]` to struct
2. Ensure nested types also derive `RkyvClone` or have R-prefixed versions
3. Add Clone, PartialEq to any enums used
4. Run tests: `cargo test --features rkyv`
5. Benchmark: `cargo bench --features rkyv`

## Examples

See:

- `formats/src/senhive/alert.rs` - Simple struct with enum
- `formats/src/senhive/fused.rs` + `rkyv.rs` - Complex nested structures (manual impl for comparison)
- `formats/benches/alert.rs` - Performance benchmarks
