# Fetiche engine code review

Reviewed on 2026-11-27. This revisits the earlier review of `fetiche-engine`, including startup, storage, tokens,
source adapters, and actor orchestration. [DESIGN.md](DESIGN.md) covers the pipeline and its recent changes in detail.

The actor layout, typed HCL configuration, and separation of source, storage, and token responsibilities remain useful.
Job execution now has bounded queues, supervised tasks, and typed outcomes. The highest-priority remaining issues are
credential logging, scheduler bookkeeping, and shutdown ownership.

## Earlier findings revisited

| Topic | Current status |
| --- | --- |
| Engine constructors | Resolved. `Engine::new` and `Engine::single` already return `Result<Self>` and delegate to `load`. Their error docs still describe startup failures as panics. [Constructors](../src/engine/startup.rs) |
| Storage registration | Partly resolved. `Storage::register` returns errors for directory creation and rotation parsing. An existing regular file can pass the existence check, and the directory error loses the original I/O cause. [Storage](../src/storage.rs) |
| Rotation syntax | Open. The parser still uses signed `i8`, casts before multiplying, and returns the unconsumed suffix. Registration accepts a successful prefix parse. [Storage](../src/storage.rs) |
| Startup validation | Partly resolved. Startup checks the config version and queue capacity and creates the run directory. Daemon worker counts and timer intervals still lack explicit validation. [Engine startup](../src/engine/startup.rs) |
| Graceful shutdown | Partly resolved. `Engine::shutdown` exists, and the runner supervises stage completion. Engine-level shutdown still sends stop signals without joining actors or acknowledging a final state sync. [Shutdown](../src/engine/lifecycle.rs), [Runner](../src/actors/runner.rs) |
| Token registration and tracing | Partly resolved. Token and storage registration already have tracing spans. Token loading still panics on unsupported entries, and some spans and source logs can expose credentials. [Tokens](../src/actors/tokens.rs), [Storage](../src/storage.rs) |
| Error typing | Partly resolved. Domain errors and typed job outcomes already coexist with `eyre`. Improve context at specific boundaries, especially storage and token I/O, before considering a crate-wide error type. [Errors](../src/error.rs), [Outcomes](../src/job.rs) |
| Feature gating and CI | Open. Default builds include FlightAware. Both the no-default-features and all-features library checks failed in this review, and CI uses the default feature set. [Cargo features](../Cargo.toml), [CI](../../.github/workflows/rust.yml) |
| Observability | Partly resolved. Jobs have identifiers and outcomes, and startup has tracing. Scheduler ticks log at info level without queue depth or worker utilization, and the resolved runtime settings have no dedicated startup summary. [Scheduler](../src/actors/scheduler.rs), [Engine startup](../src/engine/startup.rs) |
| Tests | Improved. Parser and runner regressions now cover capability errors, backpressure, failed outcomes, and cancellation. Startup, public submission, scheduler, and shutdown still lack an end-to-end test. [Parser tests](../src/engine/parse.rs), [Runner tests](../src/actors/runner.rs) |
| Listing APIs | Resolved. The engine exposes formatted source, token, and storage listings. [Catalog](../src/engine/catalog.rs), [Token methods](../src/engine/tokens.rs) |
| Documentation | Partly resolved. The README has an actor overview and the pipeline has a design document. The checked-in engine HCL still declares version 2 while the loader requires version 3, and the README attributes the state sync timer to the state actor. The scheduler owns that timer. [README](../README.md), [HCL example](../src/engine.hcl), [Scheduler](../src/actors/scheduler.rs) |

## Priority findings

### High — Source tracing can expose credentials

ASD authentication traces the returned token. The Avionix worker traces both authentication fields, and its source
setup passes credentials in a connection URL. Token storage instrumentation captures the token argument, whose
debug representation includes the value. Replace these logs with safe fields such as source name and authentication
result. Skip sensitive arguments in spans and audit debug formatting of source configs and runner messages for
transitive exposure. [ASD fetch](../src/sources/adapters/asd/fetch.rs), [Avionix worker](../src/sources/adapters/avionix/actors.rs),
[Avionix setup](../src/sources/adapters/avionix/server.rs), [Token storage](../src/actors/tokens.rs), [Token type](../src/sources/adapters/asd/token.rs)

### High — A long-running job occupies the scheduler mailbox

The tick handler moves one job to the running queue and awaits the runner's reply inside the handler. An indefinite
stream can keep later submissions and scheduler control messages waiting despite spare factory workers. The runner
publishes outcomes without sending the scheduler's `Finished` message, leaving completed jobs in its running queue.
The existing `Finished` handler pops the front entry, and `RemoveById` uses the ID as a deque index. Dispatch without
awaiting execution in the mailbox handler and use job-ID keyed completion and removal. Test out-of-order completion
and the responsiveness of scheduler commands while a stream is active.
[Scheduler](../src/actors/scheduler.rs), [Runner](../src/actors/runner.rs)

### High — Shutdown and workspace resources lack per-engine ownership

The shutdown method and the destructor send stop signals through the same process group. Because `Engine` is cloneable,
dropping any clone can request stops for the shared actors. Explicit shutdown removes the running marker without
awaiting in-flight work, actor exits, or a final state write. Startup also creates the marker and starts actors before
all later initialization steps have succeeded. Give lifecycle ownership to one shared owner, roll back partial startup,
and stop admission before draining or cancelling jobs and awaiting final state sync.

`submit_job_and_wait` also changes the process-wide working directory across an async wait. Concurrent callers can
resolve relative paths against another job's workspace, and dropping the waiting future bypasses the explicit restore.
Resolve stage paths against a job workspace instead. [Engine startup](../src/engine/startup.rs), [Shutdown and sync](../src/engine/lifecycle.rs),
[Submission and destructor](../src/engine/jobs.rs), [State startup and sync](../src/actors/state.rs)

### Medium — Reject invalid configuration before execution

- Validate daemon worker count and positive tick and sync intervals before starting actors. Log the effective settings
  with structured fields. Update the checked-in HCL to the current schema and cover it with a configuration test.
  [Engine startup](../src/engine/startup.rs), [HCL example](../src/engine.hcl)
- Parse the entire storage rotation string using an unsigned value and checked conversion to seconds. For example,
  the current parser can accept the `1h` prefix of `1hgarbage`, while signed values either wrap on cast or overflow
  during multiplication. Define whether zero is allowed. Verify storage locations are directories and retain filesystem
  error context. Add boundary and failure cases alongside the existing positive tests. [Storage](../src/storage.rs)
- Return typed errors for unsupported token entries and preserve I/O errors on lookup. Registration currently panics
  on an unexpected filename, and the synchronous registration wrapper unwraps errors.
  [Token storage](../src/actors/tokens.rs)
- Reject unsupported HCL stages before marking a job ready. `Convert` reaches `unimplemented!()`, and `Archive`
  produces an invalid consumer that the runner rejects later. A numeric `Read` value can deserialize as an HCL
  expression string. Define whether such expressions are supported before adding a strict path type.
  [Parser](../src/engine/parse.rs), [Runner](../src/actors/runner.rs)

### Medium — Define Senhive's delivery guarantee

The data feeds acknowledge messages before conversion and before the bounded pipeline send. A conversion error or
cancellation while waiting for queue capacity can therefore leave a broker-acknowledged delivery absent from the
output. Decide which event constitutes acceptance. An acknowledgement after channel send covers local enqueue.
A guarantee through persistence needs a sink acknowledgement and a retry or deduplication policy.
[Senhive actor](../src/sources/adapters/senhive/actors.rs), [Prefetch configuration](../src/sources/adapters/senhive/feed.rs)

## Verification and next coverage

- `cargo test -p fetiche-engine --lib` passed all 125 default-feature library tests.
- `cargo check -p fetiche-engine --no-default-features --lib` failed on ASD token imports, empty source dispatch enums,
  and conversion type inference.
- `cargo check -p fetiche-engine --all-features --lib` failed in optional OpenSky and SafeSky code. This also blocks
  the all-features configuration requested by docs.rs. [Cargo metadata](../Cargo.toml)

Prioritize a temporary-workspace smoke test that loads configuration, starts the engine, parses and submits a local
file job, checks the outcome and output, and shuts down. Add scheduler tests for simultaneous work and keyed
completion. Restore both feature configurations before requiring them in CI. Live sources and shutdown timing were
reviewed from code and were outside these test runs.
