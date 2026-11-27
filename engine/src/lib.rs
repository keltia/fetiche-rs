//! Library implementing common part of the transformations
//!
//! # Config
//!
//! v2: Has basedir and storage components.
//! v2: Has initial number of workers for the runner factory.
//!
//! Example:
//! ```rust,no_run
//! # #[tokio::main]
//! # async fn main() -> eyre::Result<()> {
//! use tracing::trace;
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
pub use auth::*;
pub use config::EngineConfig;
pub use consumer::*;
pub use engine::{Engine, EngineMode, Freq};
pub use error::*;
pub use job::*;
pub use middle::*;
pub use producer::*;
pub use sources::*;
pub use storage::*;
pub use task::*;
pub use workspace::*;

use fetiche_common::{Filter, time};

mod actors;
mod auth;
mod config;
mod consumer;
mod engine;
mod error;
mod job;
mod middle;
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
