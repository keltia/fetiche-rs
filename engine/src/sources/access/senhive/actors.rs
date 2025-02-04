//! Module that implement the Actors.
//!
//! We currently have only one actor: `Worker`.
//!

use std::fmt::Debug;
use std::sync::mpsc::Sender;

use futures_util::stream::StreamExt;
use lapin::{options::BasicAckOptions, Connection, ConnectionProperties};
use ractor::{call, Actor, ActorProcessingErr, ActorRef};
use tracing::{error, trace, warn};

use super::{from_json_to_csv, from_json_to_nl};
use crate::actors::StatsMsg;
use crate::{Feed, Stats};

/// This is the worker that will consume a given topic.
///
/// 1. connect to both topic and its dead letter one
/// 2. listen and consume both, knowing that we might
///    get interleaved packets from both but mainly dl_topic first
/// 3. we also subscribe to the `alert` topic, just in case.
///
/// We currently ignore the `system_state` topic.
///
pub(crate) struct Worker;

/// Contains the connection handle and the output stream.
/// We also have the address of the stat gathering actor.
///
#[derive(Debug)]
pub(crate) struct WorkerState {
    /// Connection to the AMQP server
    pub conn: Connection,
    /// Main topic we are subscribed to.
    pub topic: Option<String>,
    /// Channel to send data packets to
    pub out: Sender<String>,
    /// Who to send statistics-related events to
    pub stat: ActorRef<StatsMsg>,
}

/// How to start a Worker actor, regardless of topic(s) involved.
///
#[derive(Debug)]
pub struct WorkerArgs {
    /// URL to connect to
    pub url: String,
    /// Where to send the data fetched
    pub out: Sender<String>,
    /// For each packet, send statistics data
    pub stat: ActorRef<StatsMsg>,
}

/// This is a more or less one-task actor.
///
#[derive(Debug)]
pub(crate) enum WorkerMsg {
    /// Consume a given topic and assign it a tag
    Consume(String, String),
}

/// Worker Actor.
///
/// Do we want one actor for all topics or one actor per topic?
///
#[ractor::async_trait]
impl Actor for Worker {
    type Msg = WorkerMsg;
    type State = WorkerState;
    type Arguments = WorkerArgs;

    /// Connect to the given server and save the initial state.
    ///
    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("Starting worker actor {}", myself.get_cell().get_id());

        // Do the connection
        //
        trace!("tcp::connect");
        let conn = Connection::connect(&args.url, ConnectionProperties::default())
            .await
            .expect("connect failed");

        Ok(WorkerState {
            conn,
            topic: None,
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
    #[tracing::instrument(skip(self, myself))]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
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
            WorkerMsg::Consume(topic, tag) => {
                state.topic = Some(topic.clone());

                // Subscribe to both dl_<topic> and <topic>
                //
                let dl = format!("dl_{topic}");
                let mut data = Feed::new(&state.conn, &topic, &tag).await?;
                let mut dl_data = Feed::new(&state.conn, &dl, &tag).await?;

                // Also subscribe to the alert topic, just in case.
                //
                let mut alert = Feed::new(&state.conn, "system_alert", "oob").await?;

                eprintln!("d: dl_fused_data - D: fused_data - A: alert.");
                // Process each message
                //
                loop {
                    tokio::select! {
                        // This is for regular events, one data packet at a time
                        //
                        Some(data) = data.inp.next() => {
                            eprint!("D");
                            let delivery = data?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            // Save as a csv with `DronePoint`s
                            //
                            let data = from_json_to_csv(&delivery.data)?;
                            let len = data.len() as u64;
                            trace!("data: size={len}");

                            stat.cast(StatsMsg::Bytes(len))?;
                            stat.cast(StatsMsg::Pkts(1))?;

                            out.send(data)?;
                        },
                        // This drains the `dl_fused_data` topic, we expect this to happen upon startup.
                        //
                        Some(data) = dl_data.inp.next() => {
                            eprint!("d");
                            let delivery = data?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            // Save as a csv with `DronePoint`s
                            //
                            let data = from_json_to_csv(&delivery.data)?;
                            let len = data.len() as u64;
                            trace!("drain: size={len}");

                            stat.cast(StatsMsg::Bytes(len))?;
                            stat.cast(StatsMsg::Pkts(1))?;

                            out.send(data.clone())?;
                        },
                        // Rest is just handling events.
                        //
                        // FIXME: do we stop when getting an alert?
                        //
                        Some(alert) = alert.inp.next() => {
                            eprint!("A");
                            let delivery = alert?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            let data = from_json_to_nl(&delivery.data)?;
                            stat.cast(StatsMsg::Error)?;

                            warn!("alert={}", data);
                        },
                        else => {
                                error!("Unknown event, stopping.");
                                myself.kill();
                            },
                    }
                }
            }
        }
    }
}
