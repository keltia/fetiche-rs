//! This is the simplest scheduler ever.
//!
use futures_util::SinkExt;
use ractor::factory::{FactoryMessage, JobOptions};
use ractor::{call, cast, factory, Actor, ActorProcessingErr, ActorRef};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::actors::{QueueMsg, ResultsMsg, RunnerMsg, StateMsg};
use crate::{SchedulerError, Stats};

const TICK: Duration = Duration::from_secs(2);
const SYNC: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub enum SchedulerMsg {
    Start,
    Suspend,
    Stop,
    Tick,
}

#[derive(Debug, Default, PartialEq)]
enum Mode {
    Exiting,
    #[default]
    Idle,
    Starting,
    Running,
    Suspended,
}

#[derive(Debug)]
pub struct SchedulerArguments {
    pub state: ActorRef<StateMsg>,
    pub queue: ActorRef<QueueMsg>,
    pub results: ActorRef<ResultsMsg>,
    pub factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
}

#[derive(Debug)]
pub struct SchedulerState {
    mode: Mode,
    state: ActorRef<StateMsg>,
    queue: ActorRef<QueueMsg>,
    results: ActorRef<ResultsMsg>,
    factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
}

pub struct SchedulerActor;

#[ractor::async_trait]
impl Actor for SchedulerActor {
    type Msg = SchedulerMsg;
    type State = SchedulerState;
    type Arguments = SchedulerArguments;

    #[tracing::instrument(skip(self, _myself, args))]
    async fn pre_start(&self, _myself: ActorRef<Self::Msg>, args: Self::Arguments) -> Result<Self::State, ActorProcessingErr> {
        // We start in `Starting` state.
        //
        Ok(SchedulerState {
            mode: Mode::Starting,
            state: args.state.clone(),
            queue: args.queue.clone(),
            results: args.results.clone(),
            factory: args.factory.clone(),
        })
    }

    #[tracing::instrument(skip(self, myself))]
    async fn handle(&self, myself: ActorRef<Self::Msg>, message: Self::Msg, state: &mut Self::State) -> Result<(), ActorProcessingErr> {
        match message {
            SchedulerMsg::Start => {
                // Starting -> Idle
                //
                if state.mode == Mode::Starting {
                    myself.send_interval(TICK, || SchedulerMsg::Tick);
                    state.mode = Mode::Idle;
                    return Ok(());
                }
                error!("Scheduler was not started");
                return Err(SchedulerError::WrongState.into());
            }
            SchedulerMsg::Suspend => {
                // Idle -> Suspend
                //
                if state.mode == Mode::Idle {
                    state.mode = Mode::Suspended;
                } else {
                    error!("Scheduler is not idle");
                    return Err(SchedulerError::WrongState.into());
                }
            }
            SchedulerMsg::Stop => {
                // Regardless of our state, we are done.
                // -> Exiting
                state.mode = Mode::Exiting;
                warn!("Scheduler exiting.");
                myself.kill();
            }
            SchedulerMsg::Tick => {
                // Idle -> Running
                //
                info!("tick");
                if state.mode == Mode::Idle {
                    state.mode = Mode::Running;

                    // If all queues are empty, we are done.
                    //
                    if call!(state.queue, |port| QueueMsg::Empty(port))? {
                        state.mode = Mode::Idle;
                        myself.kill();
                    }
                    let _ = state.factory.call(
                        |port| {
                            FactoryMessage::Dispatch(factory::Job {
                                key: 0,
                                msg: RunnerMsg::Run(port),
                                options: JobOptions::default(),
                                accepted: None,
                            })
                        },
                        None)
                        .await?;

                    state.mode = Mode::Idle;
                }
                // Ignore tick if not idle.
            }
        }
        Ok(())
    }
}
