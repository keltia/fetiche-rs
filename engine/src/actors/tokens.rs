//! Token storage and management actor implementation
//!
//! This module provides an actor-based token management system that handles:
//! - Token storage and retrieval
//! - Token listing capabilities
//! - Persistent token storage operations
//!
//! The actor maintains tokens in a storage backend and provides async message-based
//! access to token operations through the TokenMsg enum interface.
//!

use std::path::Path;

use crate::{TokenStorage, TokenType, ENGINE_PG};
use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::{info, trace};

/// Actor implementation for managing token storage and operations
///
/// Handles token retrieval, listing and persistence operations through
/// an async message-based interface.
#[derive(Clone, Debug)]
pub struct TokenActor;

/// Messages that can be sent to the TokenActor
#[derive(Debug)]
pub enum TokenMsg {
    /// Retrieve a token by its key
    Get(String, RpcReplyPort<TokenType>),
    /// List all available tokens
    List(RpcReplyPort<Vec<TokenType>>),
    /// Store a token with given path and content
    Store(String, String),
}

/// Arguments for initializing a TokenActor
#[derive(Clone, Debug)]
pub struct TokenArgs {
    /// Base path for token storage
    pub path: String,
}

#[ractor::async_trait]
impl Actor for TokenActor {
    type Msg = TokenMsg;
    type State = TokenStorage;
    type Arguments = TokenArgs;

    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);

        // Register tokens
        //
        trace!("load tokens");
        let tokens_area = Path::new(&args.path)
            .join("tokens")
            .to_string_lossy()
            .to_string();
        let tokens = TokenStorage::register(&tokens_area).await?;
        info!("{} tokens loaded", tokens.len());

        Ok(tokens)
    }

    #[tracing::instrument(skip(self, _myself))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            TokenMsg::Get(key, sender) => {
                let token = state.load(&key).await?;
                sender.send(token)?;
            }
            TokenMsg::List(sender) => {
                let list = state.list();
                sender.send(state.list())?;
            }
            TokenMsg::Store(path, store) => {
                todo!()
            }
        }
        Ok(())
    }
}
