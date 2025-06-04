Fetiche Engine: Code Review and Improvement Suggestions

Scope

- This review focuses on the fetiche-engine crate: its initialization flow (Engine), storage handling, actor
  orchestration, and related modules referenced from lib.rs (actors, storage, tokens, sources, scheduler/factory
  integration).
- The aim is to provide actionable, incremental improvements with low-risk changes first, followed by medium/longer-term
  refactors.

Executive summary

- Strengths
    - Clear actor-based architecture with ractor: supervisor, scheduler, runner factory, state, results, sources, stats.
    - Good observability via tracing and EnvFilter; structured startup logs; helpful debug/trace dumps (actors list,
      storage listing).
    - Config versioning via macros (into_configfile) and strong typed config structures.
    - Separation of storage, tokens, and sources registration phases keeps boundaries clean.
- Main opportunities
    - Avoid panics in library paths; prefer Result-returning constructors and error propagation.
    - Tighten parsing and validation (e.g., rotation parsing uses i8; directory creation unwraps).
    - Provide graceful shutdown and lifecycle hooks for actors; ensure Start/Tick/Sync semantics are well-bounded.
    - Expand test coverage for configuration and storage cases; add compile-time feature matrix CI entries.
    - Clarify feature gating and default behavior to reduce unexpected compilation of unused sources.

Detailed findings and recommendations

1) Engine constructors: avoid panics, provide fallible variants (Low effort)

- Current:
    - Engine::new() and Engine::single() return Self and panic on error via unwrap_or_else + panic.
    - Engine::load() is async and returns Result<Self> but is only used indirectly.
- Suggestion:
    - Add try_new() -> Result<Self> and try_single() -> Result<Self> that call load and return errors to the caller.
      Keep existing new()/single() as convenience wrappers (possibly #[deprecated] in future).
    - Benefit: binaries (e.g., fetiched) can decide whether to retry, log, or exit with a specific code instead of hard
      panic.

2) Storage::register: remove panics and unwraps (Low/Medium effort)

- Current:
    - Directory creation uses std::fs::create_dir_all(path).unwrap_or_else(... panic!).
    - Rotation parsing uses Self::parse_rotation(...).unwrap().
- Suggestion:
    - Change register to return Result<Self> and propagate file-system and parsing errors.
    - Alternatively, add register_strict() -> Result<Self> and keep register() that logs and skips invalid entries.
    - Benefit: robust startup that can report misconfiguration without crashing the whole engine.

3) Rotation parsing: use u32 parser and wider numeric range (Low effort)

- Current:
    - parse_rotation uses nom::character::complete::i8 which caps numeric value at 127 and then matches one_of("smhd").
    - This disallows values like "1000s" or "168h" even if valid semantically.
- Suggestion:
    - Use nom::character::complete::u32 (or u64 if needed) for the number, then multiply based on suffix:
        - map_res((u32, one_of("smhd")), |(n, tag)| -> Ok(match tag { 's' => n, 'm' => n*60, 'h' => n*3600, 'd' => n*
          86400, _ => n }))
    - Add a numeric overflow guard when converting to seconds (saturating or checked_mul with a clear error) to avoid
      wrap.
    - Extend tests: include boundary cases (e.g., 0s, 1m, 1000h, invalid suffix, missing suffix).

4) Engine initialization logging and config validation (Low effort)

- Current:
    - Checks config version vs ENGINE_VERSION (good) and logs home/workdir.
- Suggestions:
    - Log computed defaults for workers, sync, tick explicitly at info level to aid ops troubleshooting.
    - Validate that basedir exists (or create it) and that workdir path is under basedir to avoid accidental writes
      elsewhere.
    - Consider warning if workers == 0 (should be clamped to >=1) or if sync < tick (may cause excessive syncs).

5) Actor lifecycle and graceful shutdown (Medium effort)

- Current:
    - Startup spawns supervisor, stats, sources, state, results, factory, scheduler; then cast! Start to scheduler.
    - No visible shutdown path in lib.rs (might exist in actors module).
- Suggestions:
    - Define an Engine::shutdown(self) async that sends a Stop/Shutdown message to scheduler and/or supervisor, awaits
      actor handles, and flushes state (call engine.sync()).
    - If feasible, tie into Ctrl+C handling in binaries to trigger graceful shutdown (dev-dependency ctrlc exists in
      workspace).

6) Token storage and secrets handling (Low effort)

- Current:
    - Tokens are loaded from a tokens directory under config_path().
- Suggestions:
    - Add log at info/debug listing token names only (not values), and warn on malformed entries.
    - Document environment override mechanism if any (or consider adding one) for CI/testing.

7) Error typing and context (Low effort)

- Current:
    - eyre::Result is used widely. Good for top-level; within library boundaries, having a domain error (thiserror) can
      help.
- Suggestions:
    - For public API errors (Engine load/try_new, Storage register/parse_rotation), consider returning a custom
      EngineError/StorageError with thiserror and rich context.
    - Keep eyre anyhow-like flexibility for binaries.

8) Feature gating and compile-time surface (Low/Medium effort)

- Current:
    - Many sources are behind features, defaults enable aeroscope, asd, avionix, senhive; opensky/flightaware/safesky
      optional.
- Suggestions:
    - For faster dev cycles, consider default-features = [] in dev profiles or provide documented cargo aliases.
    - CI: add a minimal-features job (no-default-features + 1 source), and a full-features job to ensure both
      configurations compile and tests pass.

9) Observability: structured spans and periodic telemetry (Low effort)

- Current:
    - #[tracing::instrument] on key Engine functions; good start.
- Suggestions:
    - Add instrument to storage registration and token registration functions; include fields like counts and durations.
    - Ensure scheduler tick emits periodic heartbeat at trace/debug with queue length, worker count, pending jobs (if
      available).

10) Tests and property checks (Low/Medium effort)

- Current:
    - storage.rs already has rstest tests for parse_rotation and jiff-based relative calculations.
- Suggestions:
    - Add tests for misconfigured storage entries (invalid rotation, nonexistent path) once register returns Result.
    - Add an integration test to spin up a minimal Engine with no sources (or a mocked source via feature) to validate
      actor startup and graceful shutdown.

11) API ergonomics (Low effort)

- Suggest adding getters on Engine for commonly printed lists (tokens list, sources list) as Result<String> to ease CLI
  integration.
- Provide Engine::list_storage() -> Result<String> similar to tokens list, leveraging tabled.

12) Documentation

- Current:
    - lib.rs has solid crate-level docs and examples.
- Suggestions:
    - Add a short Architecture section in engine/README.md summarizing actors and flow, linking to this file.
    - Document EngineConfig fields in engine.hcl with examples for each storage variant.

Spot examples (based on current code)

- Engine::new()/single() panics on error:
    - Consider:
      // Non-panicking variants
      pub async fn try_new() -> eyre::Result<Self> { Self::load(ENGINE_CONFIG, EngineMode::Daemon).await }
      pub async fn try_single() -> eyre::Result<Self> { Self::load(ENGINE_CONFIG, EngineMode::Single).await }

- storage::parse_rotation uses i8 and unwraps:
    - Consider switching to u32 parser and propagate errors; add checked_mul:
      map_res((u32, one_of("smhd")), |(n, tag)| match tag { 's' => Ok(n), 'm' => n.checked_mul(60).ok_or(ParseError::
      Overflow), ... })

Quick wins checklist

- [x] Add try_new()/try_single() and update binaries to use them (optional for now).
- [x] Storage::register: return Result; replace unwraps/panics with error propagation and logging.
- [ ] storage::parse_rotation: switch i8 -> u32, add overflow guards, extend tests.
- [ ] Engine startup: log computed workers/sync/tick at info level; validate ranges.
- [ ] Add #[tracing::instrument] to storage and token registration helpers; add key-value fields in spans.
- [ ] CI: add minimal-features matrix job (if maintained in this repo’s CI).

Potential medium-term refactors

- Actor supervision strategy: define hierarchical restart policies and backoff for failing actors (if ractor supports it
  via Supervisor strategy).
- Pluggable storage backends using object_store abstraction end-to-end (today LocalFileSystem is used; generalize when
  needed).
- Introduce a DomainError enum for engine with thiserror; keep eyre for top-level to add context from binaries.

Risks and compatibility

- Changing constructor signatures would be a breaking change if new()/single() were modified; prefer adding try_*
  variants first.
- Making Storage::register return Result is a small API change; consider adding a new method name to avoid breaking
  callers.
- parse_rotation numeric widening expands accepted inputs; verify any config relies on the old cap (unlikely).

Closing notes

- The engine codebase is in good shape with solid structure and observability. The above changes are aimed at improving
  robustness (avoiding panics), configuration flexibility, and developer ergonomics, while keeping changes incremental
  and low risk.
