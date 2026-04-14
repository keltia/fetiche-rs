
# fetiche-rs — Architecture Documentation (Consolidated)

This document consolidates the architectural review, derived guidelines, and the authoritative architecture description discussed previously. It is intended to be committed as `docs/ARCHITECTURE.md`.

---

## Status

**Authoritative**  
This document describes the intended architecture of **fetiche‑rs** as implemented on the `develop` branch. It documents how the system is designed to work today.

Any change that materially contradicts this document **must be justified explicitly** in a pull request.

---

## 1. Architectural Overview

`fetiche-rs` is a framework for retrieving, transforming, and processing aeronautical surveillance data. It is intentionally not a monolithic application.

The architecture is built around three core ideas:

1. Explicit orchestration only where needed
2. Async actor-based coordination for complex workflows
3. Simple, standalone tools for simple jobs

---

## 2. High-Level Structure

```
cmd/*        → user-facing binaries
engine/*     → async actor-based orchestration layer
sources/*    → data retrieval backends
formats/*    → data models and serialization
common/*     → shared domain utilities
db-utils/*   → database operational tools (ClickHouse-specific)
```

Dependencies flow strictly downward:

```
cmd
 ↓
engine
 ↓
sources / formats / common
```

---

## 3. Binary Categories (Critical)

All binaries belong to **exactly one** of these categories.

### 3.1 Engine-backed control-plane binaries

Examples: `acutectl`, `fetiched`

Responsibilities:
- Parse CLI and configuration
- Initialise runtime and tracing
- Start the actor system (Supervisor)
- Inject policy and lifecycle control

They form the **control plane**.

---

### 3.2 Standalone domain / conversion utilities (`cmd/`)

Examples: `find-airport`, `convert-from-json`

Characteristics:
- One-shot execution
- Deterministic input → output
- No engine, scheduling, or supervision

They must remain simple and predictable.

---

### 3.3 Infrastructure / operational utilities (`db-utils/`)

Examples: ClickHouse helpers

Characteristics:
- Explicitly infrastructure-specific
- Operational / administrative tasks
- No domain or business logic

---

## 4. The Actor Engine (`fetiche-engine`)

The engine solves coordination problems using async actors (`ractor`).

### Core actors
- `Supervisor`
- `SchedulerActor`
- `SourcesActor`
- `StateActor`
- `TokenActor`
- `ResultsActor`
- `StatsActor`

Source backends provide Worker actors.

---

## 5. Actor Design Rules

- Single responsibility per actor
- Message-based communication only
- One owner of mutable state (`StateActor`)
- No blocking work inside actors

Actors orchestrate — they do not compute heavily.

---

## 6. Responsibilities by Layer

| Layer      | Responsibilities                     | Must NOT Do                          |
|------------|---------------------------------------|--------------------------------------|
| `cmd/*`    | Policy, lifecycle, configuration      | Business coordination internals      |
| `engine/*` | Scheduling, supervision, coordination | Parse CLI or decide operational policy|
| `sources`  | Fetch data                            | Cross-source orchestration             |
| `formats`  | Data models, serialization            | I/O or orchestration                  |
| `db-utils` | DB ops                                | Domain logic                          |

---

## 7. Why Not Everything Uses the Engine

Using the engine where not required increases complexity and reduces clarity. Orchestration is a tool, not a doctrine.

If a task does not require coordination or lifecycle management, it must not use the engine.

---

## 8. Testing Strategy

- Engine-backed binaries → actor integration tests
- Standalone utilities → deterministic I/O tests
- db-utils → infrastructure smoke tests

---

## 9. Architectural Guidelines

### Binary classification

Every new binary must explicitly declare which category it belongs to and why.

### Dependency rules

Dependencies must follow the downward-only direction. Reverse dependencies are forbidden.

### Change management

Any PR introducing a new binary, actor, or cross-crate dependency must explain:

1. Which category it fits in
2. Why the engine is or is not used
3. Where policy is defined
4. What new coupling is introduced

---

## 10. Guiding Principle

> **Use complexity only where coordination is required.**

Fetiche’s architecture is deliberately conservative. Its strength comes from resisting unnecessary abstraction.
