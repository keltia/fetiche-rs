//! This is the `Runner` part of the engine, that is actually executing the job.
//!
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

use fetiche_sources::Stats;

#[derive(Debug)]
pub enum RunnerMsg {
    Start(usize),
    Stop(usize),
    Stats(RpcReplyPort<Stats>),
}

pub struct RunnerActor;

pub struct RunnerState {}

impl Actor for RunnerActor {
    type Msg = RunnerMsg;
    type State = RunnerState;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        todo!()
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        todo!()
    }
}
