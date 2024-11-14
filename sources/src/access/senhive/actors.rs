//! Module that implement the Actors.
//!

use futures_util::stream::StreamExt;
use lapin::{
    options::BasicAckOptions, Connection,
    ConnectionProperties,
};
use ractor::{pg::join, Actor, ActorProcessingErr, ActorRef};
use std::sync::mpsc::Sender;
use tracing::{error, trace, warn};

use fetiche_formats::senhive::{FusedData, StateMsg};

use crate::actors::{StatsMsg, PG_SOURCES};
use crate::{DataError, Feed};

pub(crate) struct Worker;

#[derive(Debug)]
pub(crate) struct WorkerState {
    pub out: Sender<String>,
    pub stat: ActorRef<StatsMsg>,
}

impl WorkerState {
    pub fn new(out: Sender<String>, stat: ActorRef<StatsMsg>) -> Self {
        Self { out, stat }
    }
}

pub(crate) enum WorkerMsg {
    Start(String),
    Tick,
}

#[ractor::async_trait]
impl Actor for Worker {
    type Msg = WorkerMsg;
    type State = WorkerState;
    type Arguments = WorkerState;

    async fn pre_start(&self, myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        join(PG_SOURCES.into(), vec![myself.get_cell()]);
        Ok(args)
    }

    async fn handle(&self, myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        let out = state.out.clone();
        let stat = state.stat.clone();

        match message {
            WorkerMsg::Start(url) => {
                trace!("Starting worker thread");

                // Do the connection
                //
                trace!("tcp::connect");
                let conn = Connection::connect(&url, ConnectionProperties::default())
                    .await
                    .expect("connect failed");

                // Subscribe to topics
                //
                let mut data = Feed::new(&conn, "fused_data", "data").await?;
                let mut alert = Feed::new(&conn, "system_alert", "oob").await?;
                let mut state = Feed::new(&conn, "system_state", "state").await?;

                let mut dl_data = Feed::new(&conn, "dl_fused_data", "data").await?;
                let mut dl_alert = Feed::new(&conn, "dl_system_alert", "oob").await?;
                let mut dl_state = Feed::new(&conn, "dl_system_state", "state").await?;

                // Process each message
                //
                loop {
                    tokio::select! {
                        // This is for regular events, one data packet at a time
                        //
                        Some(data) = data.inp.next() => {
                            let delivery = data?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            let data = String::from_utf8_lossy(&delivery.data).to_string();
                            stat.cast(StatsMsg::Bytes(data.len() as u64))?;

                            stat.cast(StatsMsg::Pkts(1))?;

                            let _: FusedData = match serde_json::from_str(&data) {
                                Ok(pdu) => pdu,
                                Err(err) => {
                                    error!("Invalid packet: {data}: {err}");
                                    let _ = stat.cast(StatsMsg::Error);

                                    return Err(DataError::BadPacketData.into());
                                }
                            };

                            out.send(data.clone())?;
                        },
                        // This drains the `dl_fused_data` topic, we expect this to happen upon startup.
                        //
                        Some(data) = dl_data.inp.next() => {
                            let delivery = data?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            let data = String::from_utf8_lossy(&delivery.data).to_string();
                            stat.cast(StatsMsg::Bytes(data.len() as u64))?;

                            stat.cast(StatsMsg::Pkts(1))?;

                            let _: FusedData = match serde_json::from_str(&data) {
                                Ok(pdu) => pdu,
                                Err(err) => {
                                    error!("Invalid packet: {data}: {err}");
                                    let _ = stat.cast(StatsMsg::Error);

                                    return Err(DataError::BadPacketData.into());
                                }
                            };

                            out.send(data.clone())?;
                        },
                        // Rest is just handling events.
                        //
                        Some(alert) = alert.inp.next() => {
                            let delivery = alert?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            let data = String::from_utf8_lossy(&delivery.data).to_string();
                            warn!("alert={}", data);
                        },
                        Some(alert) = dl_alert.inp.next() => {
                            let delivery = alert?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            let data = String::from_utf8_lossy(&delivery.data).to_string();
                            warn!("alert={}", data);
                        },
                        // FIXME: Do we need to look at these, let alone store them?
                        //
                        Some(state) = state.inp.next() => {
                            eprint!("S");
                            let delivery = state?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;

                            let data = String::from_utf8_lossy(&delivery.data).to_string();

                            let _: StateMsg = serde_json::from_str(&data)?;
                        },
                        Some(state) = dl_state.inp.next() => {
                            eprint!("s");
                            let delivery = state?;
                            delivery
                                .ack(BasicAckOptions::default())
                                .await?;
                            let data = String::from_utf8_lossy(&delivery.data).to_string();

                            let _: StateMsg = serde_json::from_str(&data)?;
                        },
                        else => {
                                error!("Unknown event, stopping.");
                                myself.kill();
                            },
                    }
                }
            }
            WorkerMsg::Tick => {
                todo!()
            }
        }
    }
}

