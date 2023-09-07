//! This `Actor` wraps the `Engine` from `fetiche-engine` and will provide an interface to it.
//!

use actix::prelude::*;
use tracing::info;

use fetiche_engine::Engine;

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineStatus {}

#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct EngineVersion {}

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

impl Handler<EngineStatus> for EngineActor {
    type Result = ();

    #[tracing::instrument(skip(self, msg))]
    fn handle(&mut self, msg: EngineStatus, _: &mut Self::Context) -> Self::Result {
        info!("{} {}", "EngineActor", fetiche_engine::version());
    }
}

impl Handler<EngineVersion> for EngineActor {
    type Result = String;

    #[tracing::instrument(skip(self, msg))]
    fn handle(&mut self, msg: EngineVersion, _: &mut Self::Context) -> Self::Result {
        fetiche_engine::version()
    }
}
