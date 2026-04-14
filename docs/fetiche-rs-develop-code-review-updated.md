# fetiche-rs (develop) — Updated Architecture & Code Review

Repository: https://github.com/keltia/fetiche-rs  
Branch reviewed: **develop**

---

## Overview

This document is an **updated and corrected senior-level technical review** of the **fetiche-rs** Rust workspace, based on the **`develop` branch** and incorporating feedback gathered after the initial review.

This version explicitly:
- Corrects earlier omissions
- Integrates **all binaries under `cmd/`**, including *standalone tools*
- Assesses the **ClickHouse-specific utilities under `db-utils/`**
- Clarifies the architectural split between **engine-backed commands** and **leaf utilities**

The review focuses on architecture, concurrency model, async design, binaries, and long-term maintainability.

---

## 1. Workspace Architecture (Develop Branch)

`fetiche-rs` is a multi-crate Cargo workspace with a **deliberate separation of responsibility** across libraries and binaries.

### Core library crates
- `fetiche-engine` — async actor-based orchestration (ractor)
- `fetiche-formats` — data models, serialization, conversion
- `fetiche-sources` — ingestion and remote access logic
- `fetiche-common` — shared domain types and utilities
- `fetiche-macros` — procedural / derive macros

### Binary crates / commands
These binaries fall into **three distinct architectural categories**:

1. **Engine-backed control-plane binaries**
2. **Standalone domain / conversion utilities**
3. **Database-specific operational tools**

This distinction is essential to understanding the system.

---

## 2. fetiche-engine: Async Actor-Based Core

On the `develop` branch, `fetiche-engine` is implemented using **`ractor`**, a Rust async actor framework inspired by Erlang/OTP.

### Confirmed engine actors
- `Supervisor`
- `SchedulerActor`
- `SourcesActor`
- `StateActor`
- `TokenActor`
- `ResultsActor`
- `StatsActor`

### Source-specific actors
- `Worker` (Senhive)
- `Worker` (Avionix)
- `Worker` (example / tests)

### Architectural properties
- Async-first execution model
- Explicit supervision tree
- Clear ownership of mutable state
- Message-based coordination

**Conclusion**:
> `fetiche-engine` is a reactive, async orchestration layer — not a CLI framework and not a data-processing monolith.

---

## 3. Categories of Binaries in fetiche-rs

### 3.1 Engine-backed binaries (control plane)

Examples include:
- `acutectl`
- `fetiched`

**Characteristics**:
- Bootstrap the async runtime and tracing
- Instantiate the `Supervisor` actor
- Inject configuration, schedules, and policies
- Control lifecycle and shutdown semantics

These binaries form the **control plane** of the system.

---

### 3.2 Standalone binaries in `cmd/`

The binaries **`find-airport`** and **`convert-from-json`** were insufficiently covered in the initial review. They are *intentionally different* from engine-backed commands.

#### `cmd/find-airport`

**Role**:
- Domain query / lookup utility
- Read-only access to static or semi-static datasets
- Deterministic, one-shot execution

**Assessment**:
- Correctly does *not* start the engine
- Directly consumes domain and format crates
- Clean separation from orchestration and scheduling logic

This is a well-designed **leaf tool**.

---

#### `cmd/convert-from-json`

**Role**:
- One-shot or batch data conversion
- JSON → internal / Arrow / other formats
- Used for migration, backfill, or compatibility

**Assessment**:
- Appropriately avoids actor infrastructure
- Keeps conversion logic isolated and testable
- Aligns with Unix-style "do one thing" tooling

Forcing these binaries through the engine would reduce clarity and increase complexity.

---

### 3.3 ClickHouse-specific binaries in `db-utils/`

The repository also contains **two ClickHouse-focused utilities** under `db-utils/`, which were not assessed previously.

**Architectural role**:
- Operational / administrative database helpers
- Schema management, inspection, or maintenance tasks
- Explicitly tied to ClickHouse, not engine orchestration

**Assessment**:
- Correctly isolated from `fetiche-engine`
- Acknowledge infrastructure specificity explicitly
- Prevents ClickHouse concerns from leaking into core logic

**Recommendation**:
- Keep these tools minimal and infrastructure-focused
- Ensure they do not accumulate business logic
- Document them as **operational utilities**, not data pipelines

---

## 4. Corrected System Topology (Conceptual)

```
cmd/*
 ├─ acutectl            → engine-backed, actor-based control plane
 ├─ fetiched            → engine-backed daemon
 ├─ process-data        → analytics pipeline
 ├─ find-airport        → standalone domain query tool
 ├─ convert-from-json   → standalone ETL helper

db-utils/*
 └─ clickhouse-*        → database operational utilities
            ↓
engine/*
 ├─ Supervisor
 ├─ SchedulerActor
 ├─ SourcesActor
 ├─ StateActor
 ├─ TokenActor
 ├─ ResultsActor
 └─ StatsActor
```

This split is **deliberate, sound, and scalable**.

---

## 5. CI, Tooling, and Maintainability Notes

### Strengths
- Clear layering
- Async actor model used where appropriate
- Standalone tools kept intentionally simple

### Recommendations
- Explicitly document the three classes of binaries in the main README
- Avoid policy logic creeping into standalone tools
- Keep ClickHouse utilities operational, not domain-driven

Suggested CI baseline (unchanged):
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --locked
```

---

## Final Assessment (Updated)

**fetiche-rs (develop)** demonstrates a **mature and nuanced architecture**:

- Actor-based async orchestration where coordination matters
- Simple synchronous tools where orchestration does not add value
- Clean isolation of infrastructure-specific concerns

The earlier review understated the importance of **leaf binaries and db-level tools**. This updated assessment reflects the full system accurately.

**Conclusion**:
> The project shows strong architectural discipline and a healthy resistance to over-engineering.
