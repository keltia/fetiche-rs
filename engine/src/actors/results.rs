use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use std::collections::BTreeMap;

use crate::Stats;

#[derive(Debug)]
pub enum ResultsMsg {
    Submit(usize, Stats),
    Fetch(usize, RpcReplyPort<Stats>),
}

#[derive(Debug)]
pub struct ResultsState {
    list: BTreeMap<usize, Stats>,
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

    #[tracing::instrument(skip(self, _myself, state))]
    async fn handle(&self, _myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match message {
            ResultsMsg::Submit(id, stats) => {
                let _ = state.list.insert(id, stats);
            }
            ResultsMsg::Fetch(id, port) => {
                let res = state.list.get(&id).unwrap_or(&Stats::default()).clone();
                let _ = port.send(res)?;
            }
        }
        Ok(())
    }
}
