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
use log::trace;
use tracing::info;

use fetiche_engine::{Cmds, Engine};

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

impl<A, M> MessageResponse<A, M> for EngineStatus
where
    A: Actor,
    M: Message<Result = EngineStatus>,
{
    fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            let _ = tx.send(self);
        }
    }
}

impl Handler<GetStatus> for EngineActor {
    type Result = EngineStatus;

    /// Return the current status of the engine
    ///
    #[tracing::instrument(skip(self, msg))]
    fn handle(&mut self, msg: GetStatus, _: &mut Self::Context) -> Self::Result {
        info!("{} {}", "EngineActor", fetiche_engine::version());

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
    #[tracing::instrument(skip(self, msg))]
    fn handle(&mut self, msg: GetVersion, _: &mut Self::Context) -> Self::Result {
        fetiche_engine::version()
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

        let r = fetiche_engine::parse_job(&cmd);
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

        trace!("handle:res={}", res);
        res
    }
}

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

#[cfg(test)]
mod tests {
    use eyre::Result;

    use super::*;

    #[test]
    fn test_foo() -> Result<()> {
        Ok(())
    }

    #[actix_rt::test]
    async fn test_engine_version() -> Result<()> {
        let str = r##"
version = 2

basedir = "/tmp"

// Describe a local directory tree used to store files
//
storage "hourly" {
  path     = ":basedir/hourly"
  rotation = "1h"
}"##;
        let cfg: fetiche_engine::EngineConfig = hcl::from_str(str)?;
        let e = Engine::from_cfg(&cfg);
        let e = EngineActor { e };
        let e = EngineActor::default().start();

        let v = e.send(GetVersion).await?;
        assert_eq!(fetiche_engine::version(), v);
        Ok(())
    }
}
