# fetiche-rs (develop) — Async Actor-Based Rust Code Review

Repository: https://github.com/keltia/fetiche-rs  
Branch reviewed: **develop**

---

## Overview

This document provides a senior-level technical review of the **fetiche-rs** Rust workspace, focusing exclusively on the **`develop` branch**, which is where active development occurs. The review emphasizes architecture, concurrency model, async design choices, dependency strategy, and long-term maintainability.

A key correction from earlier assessments is incorporated here: **`fetiche-engine` is an async, actor-based orchestration layer built using the `ractor` framework**, not a synchronous core.

---

## 1. Workspace Architecture

`fetiche-rs` is organized as a multi-crate Cargo workspace with a clear separation of concerns.

### Core library crates
- `fetiche-engine` — orchestration and pipeline control
- `fetiche-formats` — data models and format handling
- `fetiche-sources` — data ingestion and external sources
- `fetiche-common` — shared utilities and domain types
- `fetiche-macros` — procedural and declarative macros

### Binary crates
- `acutectl`
- `process-data`
- `opensky-history`
- `adsb-to-parquet`

### Supporting assets
- CI workflows
- Documentation
- Python scripts
- Test data

**Assessment**:
- The workspace layout is clear, scalable, and appropriate for a complex data-ingestion framework.
- Libraries are cleanly separated from binaries, supporting composability and testing.

---

## 2. Core Concurrency Model: Actor-Based Async Design

### `fetiche-engine`

On the `develop` branch, `fetiche-engine` is implemented using **`ractor`**, a modern async actor framework inspired by Erlang/OTP.

Key properties of this design:
- Actors are defined with `async fn pre_start` and `async fn handle`
- Message passing is type-safe and asynchronous
- State is owned and isolated per actor
- Supervision and restart semantics are explicit

**Conclusion**:
> `fetiche-engine` is an **async-first orchestration layer**, not a synchronous engine.

This is a strong architectural choice for multi-source ingestion, backpressure control, and fault isolation.

---

## 3. Async Consistency Across the Workspace

| Crate | Async Model |
|------|------------|
| `fetiche-engine` | Async actors (`ractor`) |
| `process-data` | Async (ClickHouse, I/O) |
| `acutectl` | Async CLI + orchestration |
| `fetiche-sources` | Async I/O |
| `fetiche-formats` | Mostly CPU-bound, async-friendly |

**Assessment**:
- The system is coherently **async-first**.
- This greatly reduces impedance mismatches and avoids common hybrid async/sync pitfalls.

---

## 4. Actor-Specific Review Considerations

### 4.1 Blocking Inside Actors

Actors process messages serially. Blocking operations inside them can stall progress.

**Risk areas to audit**:
- Long-running ClickHouse queries
- CPU-heavy calculations
- Large filesystem scans

**Recommendation**:
- Treat actors as orchestrators, not workers
- Offload heavy work to dedicated async tasks or worker pools

---

### 4.2 Actor Topology & Documentation

Actor systems benefit greatly from explicit documentation.

**Recommended additions**:
- Actor hierarchy diagram (Mermaid is sufficient)
- Supervision strategy documentation
- Short `fetiche-engine/README.md` describing the actor model

This will significantly improve onboarding and long-term maintainability.

---

## 5. Dependency & Versioning Strategy

### Observations
- Heavy data ecosystem dependencies:
  - Arrow / Parquet
  - DataFusion
  - ClickHouse
- Coordinated crate versions
- Project is still in `0.x`, implying expected API evolution

### Assessment
**Strengths**:
- Explicit version documentation
- Willingness to perform major backend migrations

**Risks**:
- High churn in upstream dependencies
- Actor framework upgrades can be systemic

**Recommendations**:
- Centralize versions via `[workspace.dependencies]`
- Prefer minor-version pinning for `0.x` crates
- Explicitly document and enforce MSRV in CI

---

## 6. CI, Tooling, and Project Hygiene

### Strengths
- Active CI pipelines
- Proper licensing and contribution guidelines
- Structured release branches

### Gaps
- No published GitHub releases
- No strict Clippy gating visible
- Test strategy not clearly layered (actors vs formats vs sources)

**Recommended CI baseline**:
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test --locked
```

---

## 7. Domain Suitability (Aviation / Surveillance)

The combination of:
- Async actor orchestration
- Explicit state ownership
- Columnar data formats
- Deterministic processing pipelines

is **well aligned** with aeronautical and surveillance data processing requirements.

The use of actors for fault isolation and restartability is particularly appropriate in this domain.

---

## 8. Actionable Recommendations (Priority Ordered)

### High Priority
1. Document actor topology and supervision
2. Audit actors for blocking behavior
3. Enforce Clippy and formatting in CI

### Medium Priority
4. Add crate-level READMEs (especially `fetiche-engine`)
5. Publish tagged GitHub releases
6. Add actor-focused integration tests

### Low Priority
7. Feature-gate heavy dependencies
8. Add architectural diagrams to `docs/`

---

## Final Assessment

**fetiche-rs (develop)** represents a **solid, modern, async actor-driven Rust system**.

### Key Strengths
- Clear async-first design
- Correct use of an actor model for orchestration
- Strong alignment between architecture and problem domain

### Main Risk
- Actor misuse (blocking calls, overloaded mailboxes), which is manageable with conventions and documentation

**Conclusion**:
> The `develop` branch shows a mature and well-reasoned architecture with clear paths for further hardening and long-term sustainability.
