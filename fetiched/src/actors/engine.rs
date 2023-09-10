//! This `Actor` wraps the `Engine` from `fetiche-engine` and will provide an interface to it.
//!

use actix::dev::{MessageResponse, OneshotSender};
use actix::prelude::*;
use tracing::info;

use fetiche_engine::Engine;

// ---- Commands

#[derive(Debug, Message)]
#[rtype(result = "EngineStatus")]
pub struct GetStatus {}

#[derive(Debug, Message)]
#[rtype(result = "EngineStatus")]
pub struct EngineStatus {
    /// Runtime working area
    pub home: String,
    /// Number of jobs curretnly in queue
    pub jobs: usize,
}

impl<A, M> MessageResponse<A, M> for EngineStatus
where
    A: Actor,
    M: Message<Result = EngineStatus>,
{
    fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct GetVersion {}

// ----- The Actor

#[derive(Debug)]
pub struct EngineActor {
    pub e: Engine,
}

impl Default for EngineActor {
    #[tracing::instrument]
    fn default() -> Self {
        let e = Engine::new();
        EngineActor { e }
    }
}

impl Actor for EngineActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("Engine is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("Engine is stopped");
    }
}

impl Handler<GetStatus> for EngineActor {
    type Result = EngineStatus;

    #[tracing::instrument(skip(self, msg))]
    fn handle(&mut self, msg: GetStatus, _: &mut Self::Context) -> Self::Result {
        info!("{} {}", "EngineActor", fetiche_engine::version());

        EngineStatus {
            home: self.e.home.to_owned().to_string_lossy().to_string(),
            jobs: self.e.jobs.read().iter().len(),
        }
    }
}

impl Handler<GetVersion> for EngineActor {
    type Result = String;

    #[tracing::instrument(skip(self, msg))]
    fn handle(&mut self, msg: GetVersion, _: &mut Self::Context) -> Self::Result {
        fetiche_engine::version()
    }
}
