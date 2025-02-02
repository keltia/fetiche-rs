//! This is the `Runner` part of the engine, that is actually executing the job.
//!
use ractor::{call, pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use crate::actors::QueueMsg;
use crate::ENGINE_PG;
use fetiche_sources::Stats;

#[derive(Debug)]
pub enum RunnerMsg {
    Start(usize),
    Stop(usize),
    Stats(RpcReplyPort<Stats>),
}

pub struct RunnerActor;

pub struct RunnerArgs {
    queue: ActorRef<QueueMsg>,
    stats: ActorRef<Stats>,
}

impl Actor for RunnerActor {
    type Msg = RunnerMsg;
    type State = RunnerArgs;
    type Arguments = RunnerArgs;

    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);
        Ok(args)
    }

    #[tracing::instrument(skip(self, myself))]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            RunnerMsg::Start(n) => {
                let queue = state.queue.clone();
                let mut job = call!(queue, |port| QueueMsg::GetById(n, port)).unwrap();

                let mut data = vec![];
                Ok(job.run(&mut data).await)
            }
            RunnerMsg::Stop(n) => {
                todo!()
            }
            RunnerMsg::Stats(sender) => {
                todo!()
            }
        }
    }
}
