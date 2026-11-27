use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::collections::BTreeMap;

use crate::JobOutcome;

#[derive(Debug)]
pub enum ResultsMsg {
    Submit(JobOutcome, RpcReplyPort<()>),
    Fetch(usize, RpcReplyPort<Option<JobOutcome>>),
}

#[derive(Debug)]
pub struct ResultsState {
    list: BTreeMap<usize, JobOutcome>,
}

pub struct ResultsActor;

#[ractor::async_trait]
impl Actor for ResultsActor {
    type Msg = ResultsMsg;
    type State = ResultsState;
    type Arguments = ();

    #[tracing::instrument(skip(self, _myself, _args))]
    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, _args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        Ok(ResultsState {
            list: BTreeMap::new(),
        })
    }

    #[tracing::instrument(skip(self, _myself, state, message))]
    async fn handle(&self, _myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match message {
            ResultsMsg::Submit(outcome, ack) => {
                state.list.insert(outcome.id, outcome);
                let _ = ack.send(());
            }
            ResultsMsg::Fetch(id, port) => {
                let _ = port.send(state.list.get(&id).cloned());
            }
        }
        Ok(())
    }
}
