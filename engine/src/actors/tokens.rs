//!
//!

use crate::ENGINE_PG;
use eyre::eyre;
use fetiche_sources::{init_sources_runtime, Context, Site, Sources};
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

#[derive(Clone, Debug)]
pub struct TokenActor;

pub struct TokenState {}

#[derive(Debug)]
pub enum TokenMsg {
    Get(String),
    List,
    Store(String, String),
}

#[derive(Clone, Debug)]
pub struct TokenArgs {
    pub path: String,
}

#[ractor::async_trait]
impl Actor for TokenActor {
    type Msg = TokenMsg;
    type State = TokenState;
    type Arguments = TokenArgs;

    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![_myself.get_cell()]);

        // Register tokens
        //
        trace!("load tokens");
        let tokens_area = cfg.basedir.join("tokens").to_string_lossy().to_string();
        let tokens = TokenStorage::register(&tokens_area);
        info!("{} tokens loaded", tokens.len());

        Ok(())
    }

    async fn handle(&self, _myself: ActorRef<Self::Msg>, _message: Self::Msg, _state: &mut Self::State) -> Result<(), ActorProcessingErr> {}
}

