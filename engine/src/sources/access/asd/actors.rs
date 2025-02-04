//! Module that implement the Actors.
//!
//! We currently have only one actor: `AsdActor`.
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use ractor::{pg, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::trace;

use crate::actors::StatsMsg;
use crate::{Asd, Capability, Site};

/// This is the worker that will consume a given topic.
///
/// 1. connect to both topic and its dead letter one
/// 2. listen and consume both, knowing that we might
///    get interleaved packets from both but mainly dl_topic first
/// 3. we also subscribe to the `alert` topic, just in case.
///
/// We currently ignore the `system_state` topic.
///
pub(crate) struct AsdActor;

/// Contains the connection handle and the output stream.
/// We also have the address of the stat gathering actor.
///
#[derive(Debug)]
pub(crate) struct WorkerState {
    /// Connection
    pub conn: reqwest::Client,
    /// Channel to send data packets to
    pub out: Sender<String>,
    /// Who to send statistics-related events to
    pub stat: ActorRef<StatsMsg>,
}

/// How to start a Worker actor, regardless of topic(s) involved.
///
#[derive(Debug)]
pub struct WorkerArgs {
    /// Source name
    pub name: String,
    /// Where to send the data fetched
    pub out: Sender<String>,
    /// For each packet, send statistics data
    pub stat: ActorRef<StatsMsg>,
}

/// This is a more or less one-task actor.
///
#[derive(Debug)]
pub(crate) enum WorkerMsg {
    /// Capabilities
    Capabilities(RpcReplyPort<Capability>),
    /// Consume
    Fetch(Site),
}

/// Worker Actor.
///
/// Do we want one actor for all topics or one actor per topic?
///
#[ractor::async_trait]
impl Actor for AsdActor {
    type Msg = WorkerMsg;
    type State = WorkerState;
    type Arguments = WorkerArgs;

    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("Starting worker actor {}", myself.get_cell().get_id());

        // Do the connection
        //
        trace!("create client");
        let client = reqwest::Client::new();

        Ok(WorkerState {
            conn: client,
            out: args.out,
            stat: args.stat,
        })
    }

    /// Message handler.
    ///
    /// The only interesting message is Consume() with a topic name and a topic tag.
    /// We subscribe to both the main and dead letter topic and to the alert one, just in case.
    /// Every packet received is converted into a `DronePoint` and saved as CSV, easier to store
    /// inside a DB.
    ///
    #[tracing::instrument(skip(self, _myself))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        let out = state.out.clone();
        let stat = state.stat.clone();

        match message {
            // Consuming a topic has several parallel steps:
            // - subscribing to both `<topic>` and `dl_<topic>` to ensure we get both stored and
            //   current data
            // - subscribing to the `alert` topic
            //
            WorkerMsg::Capabilities(reply) => {
                let _ = reply.send(Capability::Fetch);
                Ok(())
            }
            WorkerMsg::Fetch(site) => {
                let asd = Asd::new().load(&site).build();

                let data = asd.fetch().await?;
            }
        }
    }
}
