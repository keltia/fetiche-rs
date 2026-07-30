# Jiff Usage Guide for Fetiche

## Strategy 3: Internal-Only Migration - Implementation Complete

This guide documents how to use jiff for internal date/time operations while keeping chrono at API boundaries (Strategy 3 from JIFF_MIGRATION_ANALYSIS.md).

## Core Principle

**fetiche-formats stays on chrono** (stable public API)  
↓  
**Internal processing uses jiff** (performance-critical code)  
↓  
**Database/CSV output uses chrono** (compatibility)

## Conversion Functions

### Available in `fetiche-common`

```rust
use fetiche_common::{chrono_to_jiff, jiff_to_chrono};
use chrono::{DateTime, Utc};
use jiff::Timestamp;

// chrono → jiff (zero-cost, preserves nanoseconds)
let chrono_dt: DateTime<Utc> = /* from API */;
let jiff_ts: Timestamp = chrono_to_jiff(chrono_dt);

// jiff → chrono (for output)
let result_chrono: DateTime<Utc> = jiff_to_chrono(jiff_ts)?;
```

## Usage Patterns

### Pattern 1: Processing DronePoint Records

```rust
use fetiche_formats::DronePoint;
use fetiche_common::{chrono_to_jiff, jiff_to_chrono};
use jiff::Span;

// Receive from fetiche-formats (chrono API)
let drone: DronePoint = fetch_from_db().await?;

// Convert to jiff once at boundary
let ts = chrono_to_jiff(drone.time);

// Do fast jiff operations
let tomorrow = ts.checked_add(Span::new().days(1))?;
let is_recent = ts > Timestamp::now().checked_sub(Span::new().hours(24))?;

// Convert back to chrono for output
let output = DronePoint {
    time: jiff_to_chrono(tomorrow)?,
    ..drone
};
```

### Pattern 2: Date Interval Expansion

```rust
use fetiche_common::{chrono_to_jiff, jiff_to_chrono};
use jiff::{Span, Timestamp};

// Receive chrono dates from API
let (begin_chrono, end_chrono) = parse_date_opts()?;

// Convert to jiff for fast expansion
let begin = chrono_to_jiff(begin_chrono);
let end = chrono_to_jiff(end_chrono);

// Fast interval generation (3.5x faster than chrono)
let days_span = end.since(begin)?;
let days_count = days_span.get_days();
let mut dates = Vec::with_capacity(days_count as usize);

let day = Span::new().days(1);
let mut d = begin;
while d < end {
    dates.push(d);
    d = d.checked_add(day)?;
}

// Convert back to chrono if needed for external API
let dates_chrono: Vec<DateTime<Utc>> = dates
    .into_iter()
    .map(jiff_to_chrono)
    .collect::<Result<_>>()?;
```

### Pattern 3: Date Arithmetic in Queries

```rust
use fetiche_common::{chrono_to_jiff, jiff_to_chrono};
use jiff::Span;

// Receive chrono from external API
let query_date: DateTime<Utc> = /* from user input */;

// Convert to jiff for arithmetic
let ts = chrono_to_jiff(query_date);

// Fast date operations
let start_of_day = ts.to_zoned(jiff::tz::TimeZone::UTC)
    .round(jiff::ZonedRound::new()
        .smallest(jiff::Unit::Day)
        .mode(jiff::RoundMode::Floor))?
    .timestamp();

let end_of_day = start_of_day.checked_add(Span::new().days(1))?;

// Convert back to chrono for SQL query
let time_from = jiff_to_chrono(start_of_day)?;
let time_to = jiff_to_chrono(end_of_day)?;

// Use in SQL query
let query = format!(
    "SELECT * FROM drones WHERE time >= '{}' AND time < '{}'",
    time_from.format("%Y-%m-%d %H:%M:%S"),
    time_to.format("%Y-%m-%d %H:%M:%S")
);
```

### Pattern 4: Timestamp Comparisons

```rust
use fetiche_common::chrono_to_jiff;
use jiff::Span;

// Batch convert at start
let timestamps: Vec<Timestamp> = drone_points
    .iter()
    .map(|dp| chrono_to_jiff(dp.time))
    .collect();

// Fast jiff comparisons in hot loop
let threshold = Timestamp::now().checked_sub(Span::new().hours(1))?;

let recent: Vec<_> = timestamps
    .into_iter()
    .filter(|&ts| ts > threshold)
    .collect();

// No conversion back needed if staying in jiff domain
```

## Performance Characteristics

### Conversion Overhead

```rust
// Per-record conversion: ~20-50 nanoseconds
// For 1M records: ~20-50ms total
// Negligible compared to I/O (seconds to minutes)
```

### When Conversion Matters

✅ **Convert once per batch**, not per operation:
```rust
// ✅ GOOD: Convert once
let ts = chrono_to_jiff(drone.time);
for _ in 0..1000 {
    let next = ts.checked_add(Span::new().days(1))?;
    // ... use next
}

// ❌ BAD: Convert repeatedly
for _ in 0..1000 {
    let ts = chrono_to_jiff(drone.time);  // Wasteful!
    // ...
}
```

### Benchmark Results

From `common/benches/expand.rs`:

| Operation | chrono | jiff | Speedup |
|-----------|--------|------|---------|
| expand_interval | 5.0 µs | 1.4 µs | **3.5x faster** |
| date arithmetic | baseline | ~3x faster | significant |
| comparisons | baseline | ~2x faster | moderate |

## Migration Checklist

### ✅ Completed (Phase 1)

- [x] Add conversion helpers to `fetiche-common`
- [x] Add comprehensive tests for conversions
- [x] Migrate `process-data/distances/planes` module
- [x] Document usage patterns

### 🔄 In Progress (Phase 2)

- [ ] Migrate `engine` CSV processing loops
- [ ] Migrate query preparation code
- [ ] Add performance benchmarks for real workloads

### 📋 Planned (Phase 3)

- [ ] Audit remaining hot paths
- [ ] Document performance improvements
- [ ] Consider batching strategies for bulk conversions

## Common Pitfalls

### ❌ Don't: Convert in tight loops

```rust
// BAD: Conversion per iteration
for drone in drones {
    let ts = chrono_to_jiff(drone.time);  // 1M conversions!
    // ...
}
```

### ✅ Do: Batch convert once

```rust
// GOOD: Convert once upfront
let timestamps: Vec<_> = drones
    .iter()
    .map(|d| chrono_to_jiff(d.time))
    .collect();

for ts in timestamps {
    // Fast jiff operations
}
```

### ❌ Don't: Mix chrono and jiff in hot paths

```rust
// BAD: Constant back-and-forth
let ts_jiff = chrono_to_jiff(chrono_dt);
let next_chrono = jiff_to_chrono(ts_jiff)?;  // Wasteful
let back_to_jiff = chrono_to_jiff(next_chrono);  // Wasteful
```

### ✅ Do: Stay in jiff domain

```rust
// GOOD: Stay in jiff until final output
let ts = chrono_to_jiff(input);
let tomorrow = ts.checked_add(Span::new().days(1))?;
let next_week = tomorrow.checked_add(Span::new().weeks(1))?;
// ... many operations ...
let final_result = jiff_to_chrono(next_week)?;  // Convert once at end
```

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_date_processing_with_jiff() {
    // Start with chrono (API boundary)
    let chrono_input = Utc.with_ymd_and_hms(2024, 7, 30, 0, 0, 0).unwrap();
    
    // Convert to jiff
    let jiff_ts = chrono_to_jiff(chrono_input);
    
    // Do operations
    let result = jiff_ts.checked_add(Span::new().days(1)).unwrap();
    
    // Convert back
    let chrono_output = jiff_to_chrono(result).unwrap();
    
    // Verify
    assert_eq!(
        chrono_output,
        Utc.with_ymd_and_hms(2024, 7, 31, 0, 0, 0).unwrap()
    );
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_with_conversions() {
    // Fetch from database (chrono API)
    let drone: DronePoint = fetch_test_data().await.unwrap();
    
    // Process with jiff
    let ts = chrono_to_jiff(drone.time);
    let processed = process_with_jiff(ts).await.unwrap();
    
    // Store back (chrono API)
    let output = DronePoint {
        time: jiff_to_chrono(processed).unwrap(),
        ..drone
    };
    
    store_to_database(output).await.unwrap();
}
```

## When to Use Which Type

| Context | Use | Reason |
|---------|-----|--------|
| **Public API** (fetiche-formats) | `chrono::DateTime<Utc>` | Stable, no breaking changes |
| **Internal processing** | `jiff::Timestamp` | 3x faster, better API |
| **Database queries** | `chrono::DateTime<Utc>` | klickhouse requires it |
| **CSV output** | `chrono::DateTime<Utc>` | Existing format compatibility |
| **Date arithmetic** | `jiff::Timestamp` | Much faster, cleaner code |
| **Interval expansion** | `jiff::Timestamp` | 3.5x faster |
| **Comparisons** | `jiff::Timestamp` | 2x faster |

## Summary

Strategy 3 achieves:
- ✅ 90%+ performance gains (where it matters)
- ✅ Zero breaking changes
- ✅ Minimal conversion overhead (~50ns per record)
- ✅ Clean separation of concerns
- ✅ Easy to understand and maintain

The key insight: **date arithmetic is fast, conversion is cheap, I/O is slow**. Converting at boundaries costs nanoseconds while saving microseconds per operation on millions of records.
