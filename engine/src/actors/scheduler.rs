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

use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use ractor::factory::{FactoryMessage, JobOptions};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, factory};
use tokio::sync::oneshot;
use tracing::{error, info, trace, warn};

use crate::actors::{RunnerMsg, StateMsg};
use crate::{EngineStatus, Job, JobState, SchedulerError, WaitGroup, Work};

/// Messages that can be sent to control the scheduler's operation
#[derive(Debug)]
pub enum SchedulerMsg {
    // Basic operations
    Start,
    Suspend,
    Stop,
    Sync,
    Tick,
    // Queue operations
    /// Adds a new job to the waiting queue.
    Add(Job, RpcReplyPort<Result<WaitGroup, SchedulerError>>),
    /// Gets the next available job ID. Returns the ID through the reply port.
    Allocate(RpcReplyPort<usize>),
    /// Reports whether there are no waiting or dispatched jobs.
    Empty(RpcReplyPort<bool>),
    /// Releases the running entry for this job ID.
    Finished(usize),
    /// Lists dispatched jobs in job ID order.
    List(RpcReplyPort<Vec<usize>>),
    /// Removes a waiting job by ID and reports whether it was removed.
    RemoveById(usize, RpcReplyPort<Result<(), EngineStatus>>),
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
    pub factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
}

/// Internal state maintained by the scheduler actor
#[allow(dead_code)]
#[derive(Debug)]
pub struct SchedulerState {
    mode: Mode,
    sync: Duration,
    tick: Duration,
    last: usize,

    // Actors
    state: ActorRef<StateMsg>,
    factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,

    // The queues
    waiting: VecDeque<Work>,
    running: BTreeSet<usize>,
}

impl SchedulerState {
    /// Queues a ready job and creates its completion receiver. Only one outstanding
    /// job can use a given ID.
    fn enqueue(&mut self, job: Job) -> Result<WaitGroup, SchedulerError> {
        let id = job.id;
        if job.state() != JobState::Ready {
            return Err(SchedulerError::JobNotReady(id));
        }
        if self.running.contains(&id) || self.waiting.iter().any(|work| work.id() == id) {
            return Err(SchedulerError::DuplicateJob(id));
        }
        let (tx, rx) = oneshot::channel();
        trace!(job_id = id, "Adding job to waiting queue");
        self.waiting.push_back(Work::new(job, tx));
        Ok(WaitGroup::new(id, rx))
    }

    /// Drops a waiting job by ID, notifying its waiter of cancellation.
    /// A dispatched job owns a runner slot until it finishes.
    fn remove_waiting(&mut self, id: usize) -> Result<(), EngineStatus> {
        if self.running.contains(&id) {
            return Err(EngineStatus::JobIsRunning(id));
        }
        let index = self
            .waiting
            .iter()
            .position(|work| work.id() == id)
            .ok_or(EngineStatus::JobNotQueued(id))?;
        self.waiting.remove(index);
        Ok(())
    }

    /// Retires exactly the completed job. Unknown IDs have no effect.
    fn finish(&mut self, id: usize) {
        self.running.remove(&id);
    }
}

/// Actor implementation for the scheduler that manages task execution flow
pub struct SchedulerActor;

#[ractor::async_trait]
impl Actor for SchedulerActor {
    type Msg = SchedulerMsg;
    type State = SchedulerState;
    type Arguments = SchedulerArguments;

    /// Starts with empty job queues and waits for the initial `Start` message.
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
            factory: args.factory.clone(),
            waiting: VecDeque::new(),
            running: BTreeSet::new(),
        })
    }

    /// Handles scheduler commands and applies queue changes by job ID.
    #[tracing::instrument(skip(self, myself, message, state))]
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

                    // Check the waiting queue
                    //
                    if let Some(work) = state.waiting.pop_front() {
                        // Move unto the running queue
                        //
                        let id = work.id();
                        state.running.insert(id);

                        // Hand over to the runner factory
                        //
                        let dispatch = state
                            .factory
                            .call(
                                |port| {
                                    FactoryMessage::Dispatch(factory::Job {
                                        key: id,
                                        msg: RunnerMsg::Run(work, port),
                                        options: JobOptions::default(),
                                        accepted: None,
                                    })
                                },
                                None,
                            )
                            .await;
                        state.finish(id);
                        dispatch?;
                    };
                    state.mode = Mode::Idle;
                }
                // Ignore tick if not idle.
            }
            // sync is handled here now.
            //
            SchedulerMsg::Sync => {
                let _ = state.state.cast(StateMsg::Sync);
            }
            // The real start of the scheduler, we will generate clock ticks now.
            //
            SchedulerMsg::Start => {
                // Starting -> Idle
                //
                if state.mode == Mode::Starting {
                    // Begin with a sync.
                    //
                    let _ = state.state.cast(StateMsg::Sync);

                    // Set our clocks up
                    //
                    myself.send_interval(state.tick, || SchedulerMsg::Tick);
                    myself.send_interval(state.sync, || SchedulerMsg::Sync);

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
                let _ = sender.send(state.running.iter().copied().collect());
            }

            // ----- Job operations

            // Allocate a PID for a new job
            //
            SchedulerMsg::Allocate(sender) => {
                info!("New job allocated {}", state.last);
                let id = state.last;
                state.last += 1;
                let _ = sender.send(id);
            }

            // Add a job to the waiting queue
            //
            SchedulerMsg::Add(job, port) => {
                let id = job.id;
                let result = state.enqueue(job);
                let queued = result.is_ok();
                if port.send(result).is_err() && queued {
                    let _ = state.remove_waiting(id);
                }
            }

            SchedulerMsg::Finished(id) => {
                state.finish(id);
            }

            SchedulerMsg::RemoveById(id, reply) => {
                let _ = reply.send(state.remove_waiting(id));
            }

            // Returns status of all queues.
            //
            SchedulerMsg::Empty(sender) => {
                let _ = sender.send(state.waiting.is_empty() && state.running.is_empty());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobStatus;
    use ractor::{Message, call};
    use tokio::sync::mpsc;
    use tokio::time::timeout;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    struct Forward<M>(std::marker::PhantomData<fn() -> M>);

    #[ractor::async_trait]
    impl<M: Message> Actor for Forward<M> {
        type Msg = M;
        type State = mpsc::UnboundedSender<M>;
        type Arguments = mpsc::UnboundedSender<M>;

        /// Uses the provided channel as the test actor's mailbox observer.
        async fn pre_start(
            &self,
            _myself: ActorRef<M>,
            args: Self::Arguments,
        ) -> Result<Self::State, ActorProcessingErr> {
            Ok(args)
        }

        /// Forwards messages so the test can control when a factory job finishes.
        async fn handle(
            &self,
            _myself: ActorRef<M>,
            message: M,
            state: &mut Self::State,
        ) -> Result<(), ActorProcessingErr> {
            let _ = state.send(message);
            Ok(())
        }
    }

    struct Fixture {
        scheduler: ActorRef<SchedulerMsg>,
        state: ActorRef<StateMsg>,
        factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
        dispatched: mpsc::UnboundedReceiver<FactoryMessage<usize, RunnerMsg>>,
    }

    impl Fixture {
        /// Starts a scheduler with manual ticks and a controllable factory.
        async fn new() -> Self {
            let (state_tx, _state_rx) = mpsc::unbounded_channel();
            let (state, _) = Actor::spawn(None, Forward(std::marker::PhantomData), state_tx)
                .await
                .unwrap();
            let (factory_tx, dispatched) = mpsc::unbounded_channel();
            let (factory, _) = Actor::spawn(None, Forward(std::marker::PhantomData), factory_tx)
                .await
                .unwrap();
            let (scheduler, _) = Actor::spawn(
                None,
                SchedulerActor,
                SchedulerArguments {
                    sync: Duration::from_secs(3600),
                    tick: Duration::from_secs(3600),
                    last: 1,
                    state: state.clone(),
                    factory: factory.clone(),
                },
            )
            .await
            .unwrap();
            Self {
                scheduler,
                state,
                factory,
                dispatched,
            }
        }

        /// Submits a ready job with the requested ID.
        async fn enqueue(&self, id: usize) -> Result<WaitGroup, SchedulerError> {
            let job = Job::builder().id(id).state(JobState::Ready).build();
            timeout(
                TEST_TIMEOUT,
                self.scheduler
                    .call(|port| SchedulerMsg::Add(job, port), None),
            )
            .await
            .unwrap()
            .unwrap()
            .unwrap()
        }
    }

    impl Drop for Fixture {
        /// Stops the actors owned by this test fixture.
        fn drop(&mut self) {
            self.scheduler.kill();
            self.factory.kill();
            self.state.kill();
        }
    }

    /// Removes a queued job by ID and keeps the other job and the scheduler alive.
    #[tokio::test]
    async fn remove_waiting_job_by_id() {
        let mut fixture = Fixture::new().await;
        let first = fixture.enqueue(40).await.unwrap();
        let second = fixture.enqueue(90).await.unwrap();
        let scheduler = &fixture.scheduler;

        assert!(matches!(
            fixture.enqueue(40).await,
            Err(SchedulerError::DuplicateJob(40))
        ));
        call!(scheduler, |port| SchedulerMsg::RemoveById(90, port))
            .unwrap()
            .unwrap();
        let outcome = timeout(TEST_TIMEOUT, second.rx).await.unwrap().unwrap();
        assert_eq!(outcome.id, 90);
        assert_eq!(outcome.status, JobStatus::Cancelled { stage: None });

        assert!(!call!(scheduler, SchedulerMsg::Empty).unwrap());
        assert!(matches!(
            call!(scheduler, |port| SchedulerMsg::RemoveById(90, port)).unwrap(),
            Err(EngineStatus::JobNotQueued(90))
        ));
        call!(scheduler, |port| SchedulerMsg::RemoveById(40, port))
            .unwrap()
            .unwrap();
        assert_eq!(first.rx.await.unwrap().id, 40);
        assert!(call!(scheduler, SchedulerMsg::Empty).unwrap());
        assert!(fixture.dispatched.try_recv().is_err());
    }
}
