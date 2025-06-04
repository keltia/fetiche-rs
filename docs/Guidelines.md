Fetiche-rs: Development Guidelines (project-specific)

Audience: Advanced Rust developers working on this workspace.

Scope: This document captures only project-specific practices and pitfalls that help you build, test, and extend the codebase effectively.

1. Workspace, crates, and build configuration
- Workspace layout
  - Root Cargo.toml defines a Rust workspace with multiple crates:
    - Libraries: fetiche-common, fetiche-engine, fetiche-formats, fetiche-macros
    - Binaries/tools: acutectl, process-data, compute-height, compute-localtime
    - Additional crates present but not part of default-members (e.g., opensky-history)
  - default-members are: acutectl, process-data, compute-height, compute-localtime. Use --workspace to build everything.
- Toolchain and editions
  - Edition: 2024 across crates (e.g., engine).
  - MSRV: 1.85 due to async traits and dependencies used (see README). 
- Features
  - Engine (fetiche-engine) defines feature flags for source support: aeroscope, asd, avionix, flightaware, opensky, safesky, senhive. Defaults enable: asd, senhive, avionix, aeroscope.
  - privacy and json features exist but are empty markers today; privacy is intended as a compile-time choice.
  - Logging/telemetry: common::logging supports optional telemetry via the telemetry feature on the common crate. When enabled, logs can export spans via OTLP.
- Build targets
  - Build the whole workspace (debug):
    - PowerShell: cargo build --workspace
  - Build release:
    - cargo build --workspace --release
  - Build only default-members (the main binaries):
    - cargo build
  - Build with a specific feature set (example: engine with opensky only, disabling defaults):
    - cargo build -p fetiche-engine --no-default-features --features "opensky"
  - If you need all source features at once:
    - cargo build -p fetiche-engine --features "aeroscope asd avionix flightaware opensky safesky senhive privacy json"
- Platform specifics
  - Project is cross-platform. For Windows, PowerShell is preferred. Paths and scripts in this doc assume PowerShell.
  - The Makefile is primarily for UNIX-like install flows and for the author’s environment. Prefer cargo for Windows.

2. Runtime, configuration, and environment
- Logging
  - Logging is initialized via fetiche_common::logging::init_logging(name, use_telemetry, use_tree, use_file).
  - Log level/filter configured via RUST_LOG (tracing_subscriber::EnvFilter). Examples:
    - $env:RUST_LOG = "info,fetiche_engine=debug"
    - $env:RUST_LOG = "trace"  # verbose for debugging
  - Hierarchical output tree can be enabled by passing use_tree = true to init_logging.
  - File logging can be enabled by providing use_file = Some(path) (hourly rotation), e.g., a directory path where logs will be written; filenames are derived from the application name.
- Telemetry (optional)
  - If compiled with feature telemetry (on common), OpenTelemetry OTLP export can be enabled via init_logging(..., use_telemetry = true, ...). The code uses opentelemetry-otlp with tonic. Set up your OTLP endpoint via standard otel env vars if needed.
- Data sources and features
  - fetiche-engine sources are compiled behind feature flags. Select only the ones you need to minimize compile time and dependency surface. Some sources may require credentials at runtime; keep secrets out of the repo.

3. Testing
- General
  - The workspace includes unit/it tests across crates. Running all tests may compile optional heavy dependencies (e.g., Polars). Expect a non-trivial first build.
  - Recommended developer flows:
    - Run tests for the whole workspace:
      - cargo test --workspace
    - Run tests for a specific crate:
      - cargo test -p fetiche-common
    - Run a single test target (integration test example):
      - cargo test -p fetiche-common --test smoke
    - Filter tests by name (substring):
      - cargo test -p fetiche-engine parse::  # runs tests whose names include "parse::"
- Environment and isolation
  - Tests should not depend on external network by default; if you add tests that require network or credentials, gate them with cfg flags or environment checks so default cargo test works offline.
  - For tests that require logs, you may set RUST_LOG prior to running cargo test. The logging initializer is opt-in; keep tests deterministic.
- Adding new tests
  - Unit tests: add #[cfg(test)] mod tests { ... } in the same file as the code under test.
  - Integration tests: create tests/*.rs in the corresponding crate.
  - Property tests: the workspace includes proptest in dependencies; follow standard patterns if you need fuzz-like coverage.
- Verified example (performed during preparation of this document)
  - Created a temporary integration test common\tests\smoke.rs with contents:
    - #[test]
      fn smoke_addition() { assert_eq!(2 + 2, 4); }
  - Executed:
    - cargo test -p fetiche-common --test smoke
  - Result: build succeeded and the test passed (1 passed, 0 failed). The temporary test file was then removed to keep the repo clean.

4. Benchmarks
- Some crates (e.g., engine) define Criterion benches (see [[bench]] in engine/Cargo.toml). To run benches:
  - cargo bench -p fetiche-engine
- Note: Criterion may be behind dev-dependencies; ensure you have the toolchain and runtime requirements. On Windows, long-running benches can trigger Defender scans; consider excluding target/ if safe.

5. Code style, structure, and development tips
- Style and lints
  - Follow idiomatic Rust with 2024 edition conventions. Prefer small, focused modules; the codebase uses strum, derive_builder, and other derive-heavy patterns; keep derives consistent and documented.
  - Consider running cargo fmt and cargo clippy locally. No custom rustfmt or clippy configuration is provided, so defaults apply.
- Tracing and diagnostics
  - Use tracing spans (#[tracing::instrument]) for new async or complex functions to keep observability consistent with existing code.
  - Prefer eyre::Result for application errors in higher layers; propagate context with eyre where helpful.
- Features and compilation time
  - Be intentional with feature gates for sources (engine) and telemetry (common). Reducing enabled features significantly shortens build time when working on specific areas.
- Data handling
  - The project uses Polars heavily in data-processing paths. When adding new data operations, prefer lazy APIs where possible; keep memory pressure in mind on Windows.
- Actor model
  - fetiched and parts of the engine use ractor for actor-based orchestration. When extending, keep actor boundaries explicit and message types serializable when needed.

6. Build and run examples
- Build and install primary binaries (debug):
  - cargo build
  - Binaries: target\debug\acutectl.exe, process-data.exe, compute-height.exe, compute-localtime.exe
- Build entire workspace (including libs and non-default members) for dependency checks:
  - cargo build --workspace
- Release builds:
  - cargo build --release
- Running binaries from workspace (examples):
  - cargo run -p acutectl -- --help
  - cargo run -p process-data -- --help

7. Common troubleshooting
- Missing linker/tools on Windows
  - Ensure MSVC toolchain installed via rustup (rustup show) and Visual Studio Build Tools are present.
- Slow builds
  - Reduce enabled features on fetiche-engine while iterating.
  - Use incremental builds (default in debug), and consider sccache if available.
- Logging not showing
  - Set RUST_LOG appropriately and ensure init_logging is called in your binary path with desired options.

Appendix: quick commands
- Build (workspace): cargo build --workspace
- Test (workspace): cargo test --workspace
- Test (crate): cargo test -p fetiche-common
- Run specific test: cargo test -p fetiche-common --test smoke
- Engine with minimal features: cargo build -p fetiche-engine --no-default-features --features "opensky"
