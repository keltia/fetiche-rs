//! Actor-managed execution of data jobs.
//!
//! [`Engine`] loads an [`EngineConfig`] and owns the actors for a running instance.
//! It parses and submits [`Job`]s composed of a [`Producer`], optional [`Middle`]
//! stages, and a [`Consumer`].
//!
//! Example:
//! ```rust,no_run
//! # #[tokio::main]
//! # async fn main() -> eyre::Result<()> {
//! use fetiche_engine::Engine;
//!
//! // Instantiate Engine (in daemon mode).
//! // For single usage mode, use `single()`.
//! //
//! let engine = Engine::new().await?;
//!
//! println!("Engine initialised and running.");
//! println!("{}", engine.list_tokens().await?);
//! # Ok(())
//! # }
//! ```
//!
//! In the `Engine`, we run jobs.  `Jobs` are made from a list of `Task` and all tasks are put into
//! a pipeline.  All tasks must be `Runnable` and the `RunnableDerive` proc-macro stitches everything
//! together with channels.
//!
//! Most jobs will be fetch or stream with a conversion task at the end, etc.
//! For the first task, the stdin channel will just serve as a trigger for the pipeline.
//!
//! Each `Runnable` task will be marked as `RunnableDerive` and will need to define an `execute()`
//! member function for the main task.  It takes the previous stage output as a string and should
//! return a string with the transformed output that will be sent to the next stage.
//!
pub use actors::{TokenMsg, TokenStorage};
pub use auth::{Auth, Expirable, TokenType};
pub use config::EngineConfig;
pub use consumer::{Archive, Consumer, Save, Stdout, Store};
pub use engine::{Engine, EngineMode, Freq};
pub use error::{
    EngineStatus, ParserError, Pipeline, RunnerError, SchedulerError, StateError, StatsError,
    StorageError, TokenStatus, WsError,
};
pub use job::{Job, JobBuilder, JobOutcome, JobState, JobStatus, WaitGroup, Work};
pub use middle::{Convert, Copy, Message, Middle, Nothing, Tee};
pub use producer::{Dummy, Fetch, Producer, Read, Stream};
#[cfg(feature = "aeroscope")]
pub use sources::adapters::aeroscope::Aeroscope;
#[cfg(feature = "asd")]
pub use sources::adapters::asd::Asd;
pub use sources::adapters::asd_token::AsdToken;
#[cfg(feature = "avionix")]
pub use sources::adapters::avionix::{
    AvionixServer, Cube, Worker, WorkerArgs, WorkerMode, WorkerMsg, WorkerState,
};
#[cfg(feature = "flightaware")]
pub use sources::adapters::flightaware::{Command, Compress, Events, Flightaware, Param};
#[cfg(feature = "opensky")]
pub use sources::adapters::opensky::{
    CACHE_IDLE, CACHE_MAX, CACHE_SIZE, OpenskyDevice, OpenskyServer,
};
#[cfg(feature = "safesky")]
pub use sources::adapters::safesky::Safesky;
#[cfg(feature = "senhive")]
pub use sources::adapters::senhive::Senhive;
pub use sources::{
    AccessError, AuthError, Capability, DataError, DataType, Fetchable, FetchableSource,
    ParamError, Routes, SOURCES_CONFIG, Site, Sources, SourcesConfig, Streamable, StreamableSource,
};
pub use stats::Stats;
pub use storage::{Storage, StorageConfig, StoreArea};
pub use task::{IO, Runnable, Task};
pub use workspace::{Workspace, WsItem, WsStatus};

use fetiche_common::{Filter, time};

mod actors;
mod auth;
mod config;
mod consumer;
mod engine;
mod error;
mod job;
mod middle;
mod pipeline;
mod producer;
mod sources;
mod stats;
mod storage;
mod task;
pub mod token;
mod workspace;

/// Engine signature
///
pub fn version() -> String {
    format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// Engine process group for the actors
const ENGINE_PG: &str = "engine.pg";

/// File created into our workdir to indicate something is running.
const CANARY_FILE: &str = "running";
