//! Actor maintaining the different sources we have loaded.
//!

use fetiche_sources::Sources;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

#[derive(Debug)]
pub enum SourcesMsg {
    Get(String),
    Count(RpcReplyPort<usize>),
}

pub struct SourcesActor;

#[ractor::async_trait]
impl Actor for SourcesActor {
    type Msg = SourcesMsg;
    type State = Sources;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let sources = Sources::load()?;
        Ok(sources)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        todo!()
    }
}
