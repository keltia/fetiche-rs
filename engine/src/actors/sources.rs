//! Actor maintaining the different sources we have loaded.
//!

use crate::ENGINE_PG;
use fetiche_sources::Sources;
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

#[derive(Debug)]
pub enum SourcesMsg {
    Get(String),
    Count(RpcReplyPort<usize>),
    List(RpcReplyPort<Sources>),
}

pub struct SourcesActor;

#[ractor::async_trait]
impl Actor for SourcesActor {
    type Msg = SourcesMsg;
    type State = Sources;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        let sources = Sources::new()?;
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
