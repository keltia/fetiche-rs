//! Scheduler actor implementation that manages task execution and coordination.
//!
//! This module provides a simple scheduler that:
//! - Controls the execution flow of tasks through state transitions
//! - Coordinates with queue, state, and results actors
//! - Manages runner factory for task execution
//! - Implements periodic task checking through ticks
//! - Supports start/stop/suspend operations
//!
//! The scheduler operates in different modes (Idle, Running, Suspended, etc.)
//! and coordinates task execution through message passing.
//!

use std::collections::VecDeque;
use std::sync::mpsc::channel;
use std::time::Duration;

use ractor::factory::{FactoryMessage, JobOptions};
use ractor::{factory, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing::{error, info, trace, warn};

use crate::actors::{ResultsMsg, RunnerMsg, StateMsg};
use crate::{Job, JobState, SchedulerError, WaitGroup, Work};

/// Messages that can be sent to control the scheduler's operation
#[derive(Debug)]
pub enum SchedulerMsg {
    // Basic operations
    Start,
    Suspend,
    Stop,
    Tick,
    // Queue operations
    /// Adds a new job to the waiting queue.
    Add(Job, RpcReplyPort<WaitGroup>),
    /// Gets the next available job ID. Returns the ID through the reply port.
    Allocate(RpcReplyPort<usize>),
    /// Check if there is anything to do in any of the queues.
    Empty(RpcReplyPort<bool>),
    /// Move from running into the finished queue.
    Finished(usize),
    /// Lists all job IDs currently in the queue. Returns the list of IDs through the reply port.
    List(RpcReplyPort<Vec<usize>>),
    /// Removes a job from the queue using its ID.
    RemoveById(usize),
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

/// Configuration arguments required to initialize the scheduler actor
#[derive(Debug)]
pub struct SchedulerArguments {
    pub sync: Duration,
    pub tick: Duration,
    pub last: usize,

    // Actors
    pub state: ActorRef<StateMsg>,
    pub results: ActorRef<ResultsMsg>,
    pub factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
}

/// Internal state maintained by the scheduler actor
#[derive(Debug)]
pub struct SchedulerState {
    mode: Mode,
    sync: Duration,
    tick: Duration,
    last: usize,

    // Actors
    state: ActorRef<StateMsg>,
    results: ActorRef<ResultsMsg>,
    factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,

    // The queues
    waiting: VecDeque<Work>,
    running: VecDeque<Work>,
    finished: VecDeque<Job>,
}

/// Actor implementation for the scheduler that manages task execution flow
pub struct SchedulerActor;

#[ractor::async_trait]
impl Actor for SchedulerActor {
    type Msg = SchedulerMsg;
    type State = SchedulerState;
    type Arguments = SchedulerArguments;

    #[tracing::instrument(skip(self, _myself, args))]
    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        // We start in `Starting` state.
        //
        Ok(SchedulerState {
            mode: Mode::Starting,
            sync: args.sync,
            tick: args.tick,
            last: args.last,
            state: args.state.clone(),
            results: args.results.clone(),
            factory: args.factory.clone(),
            waiting: VecDeque::new(),
            running: VecDeque::new(),
            finished: VecDeque::new(),
        })
    }

    #[tracing::instrument(skip(self, myself))]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // ----- Basic operations

            // The most important message, the one we get every clock tick
            //
            SchedulerMsg::Tick => {
                // Idle -> Running
                //
                info!("tick");
                if state.mode == Mode::Idle {
                    state.mode = Mode::Running;

                    // Loop if nothing to do
                    //
                    if state.waiting.is_empty()
                        && state.running.is_empty()
                        && state.finished.is_empty()
                    {
                        state.mode = Mode::Idle;
                        return Ok(());
                    }

                    // Check the waiting queue
                    //
                    if let Some(work) = state.waiting.pop_front() {
                        // Move unto the running queue
                        //
                        state.running.push_back(work.clone());

                        // Hand over to the runner factory
                        //
                        let _ = state
                            .factory
                            .call(
                                |port| {
                                    FactoryMessage::Dispatch(factory::Job {
                                        key: work.id(),
                                        msg: RunnerMsg::Run(work, port),
                                        options: JobOptions::default(),
                                        accepted: None,
                                    })
                                },
                                None,
                            )
                            .await?;
                    };
                    state.mode = Mode::Idle;
                }
                // Ignore tick if not idle.
            }
            // The real start of the scheduler, we will generate clock ticks now.
            //
            SchedulerMsg::Start => {
                // Starting -> Idle
                //
                if state.mode == Mode::Starting {
                    myself.send_interval(state.tick, || SchedulerMsg::Tick);
                    state.mode = Mode::Idle;
                    return Ok(());
                }
                error!("Scheduler was not started");
                return Err(SchedulerError::WrongState.into());
            }
            // Suspend if idle
            //
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
            // Shutdown
            //
            SchedulerMsg::Stop => {
                // Regardless of our state, we are done.
                // -> Exiting
                state.mode = Mode::Exiting;
                warn!("Scheduler exiting.");
                myself.kill();
            }
            // This is ps(1)
            //
            SchedulerMsg::List(sender) => {
                let list = state.running.iter().map(|j| j.id()).collect::<Vec<usize>>();
                sender.send(list)?;
            }

            // ----- Job operations

            // Allocate a PID for a new job
            //
            SchedulerMsg::Allocate(sender) => {
                info!("New job allocated {}", state.last);
                sender.send(state.last)?;
                state.last += 1;
            }

            // Add a job to the waiting queue
            //
            SchedulerMsg::Add(job, port) => {
                let queued = job.clone();
                if job.state() != JobState::Ready {
                    return Err(SchedulerError::JobNotReady(job.id).into());
                }
                trace!("Adding job to waiting queue: {:?}", queued);

                let (tx, rx) = channel();

                let work = Work::new(job.clone(), tx);
                let wg = WaitGroup::new(job.id, rx);

                // We add the job and its notification channel after completion.
                //
                state.waiting.push_back(work);
                let _ = port.send(wg)?;
            }

            // Move a job from the running queue into the finished one.
            //
            SchedulerMsg::Finished(_batch) => {
                let work = match state.running.pop_front() {
                    Some(batch) => batch,
                    None => return Ok(()),
                };
                state.finished.push_back(work.job.clone());
                work.tx.send(())?;
            }

            SchedulerMsg::RemoveById(id) => {
                state.running.remove(id);
            }

            // Returns status of all queues.
            //
            SchedulerMsg::Empty(sender) => {
                sender.send(
                    state.waiting.is_empty()
                        && state.running.is_empty()
                        && state.finished.is_empty(),
                )?;
            }
        }
        Ok(())
    }
}
