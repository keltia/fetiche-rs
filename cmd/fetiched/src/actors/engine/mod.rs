//! This `Actor` wraps the `Engine` from `fetiche-engine` and will provide an interface to it.
//!
//! API:
//!
//! - `EngineStatus`
//! - `GetVersion`
//! - `Submit`
//!

use actix::dev::{MessageResponse, OneshotSender};
use actix::prelude::*;
use eyre::Result;
use log::trace;
use std::path::PathBuf;
use tracing::info;

use crate::{engine, parse_job, response_for, version, Bus, Cmds, Engine, Sync};

// ---- Commands

/// Return the current status of the engine
///
#[derive(Debug, Message)]
#[rtype(result = "EngineStatus")]
pub struct GetStatus;

#[derive(Debug, Message)]
#[rtype(result = "EngineStatus")]
pub struct EngineStatus {
    /// Runtime working area
    pub home: String,
    /// Number of jobs currently in queue
    pub jobs: usize,
}

response_for!(EngineStatus);

impl Handler<GetStatus> for EngineActor {
    type Result = EngineStatus;

    /// Return the current status of the engine
    ///
    #[tracing::instrument(skip(self))]
    fn handle(&mut self, _msg: GetStatus, _: &mut Self::Context) -> Self::Result {
        info!("{} {}", "EngineActor", version());

        EngineStatus {
            home: self.e.home.to_owned().to_string_lossy().to_string(),
            jobs: self.e.jobs.read().iter().len(),
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct GetVersion;

impl Handler<GetVersion> for EngineActor {
    type Result = String;

    /// Return a string representing the engine version
    ///
    #[tracing::instrument(skip(self))]
    fn handle(&mut self, _msg: GetVersion, _: &mut Self::Context) -> Self::Result {
        version()
    }
}

/// Submit a new job to the engine.
///
#[derive(Debug, Message)]
#[rtype(result = "String")]
pub struct Submit(String);

impl Submit {
    pub fn new(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl Handler<Submit> for EngineActor {
    type Result = String;

    /// String is parsed as a series of commands
    ///
    #[tracing::instrument(skip(self, _ctx))]
    fn handle(&mut self, msg: Submit, _ctx: &mut Self::Context) -> Self::Result {
        let cmd = msg.0;

        let r = parse_job(&cmd);
        let (_, (cmd, arg)) = match r {
            Ok((msg, cmd)) => (msg, cmd),
            Err(e) => return e.to_string(),
        };

        trace!("cmd={}", cmd);
        if cmd != Cmds::Echo {
            unimplemented!()
        }

        trace!("msg={}", arg);

        let task = engine::Echo::new(&arg);
        let copy = engine::Copy::new();

        let mut job = self.e.create_job("handle::submit");
        job.add(Box::new(task));
        job.add(Box::new(copy));

        let mut data = vec![];

        trace!("handle::run");
        let _ = job.run(&mut data);

        let res = String::from_utf8(data).unwrap();

        trace!("Remove job({})", job.id);
        self.e.remove_job(job);

        trace!("Sync.");
        let _ = self.e.state.do_send(Sync);

        trace!("handle:res={}", res);
        res
    }
}

// ----- The Actor

#[derive(Debug)]
pub struct EngineActor {
    pub e: Engine,
}

impl EngineActor {
    #[tracing::instrument(skip(bus))]
    pub async fn new(workdir: &PathBuf, bus: &Bus) -> Self {
        let e = Engine::new(workdir, &bus).await;
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
