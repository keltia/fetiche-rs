//! Dispatches queued jobs to available runner slots without waiting in the mailbox.
//!
//! A tick fills available slots in FIFO order. Completion releases a slot by job
//! ID and can dispatch the next waiting job immediately. Suspension pauses new
//! dispatches while allowing current jobs to complete and control messages to run.

use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use ractor::factory::{FactoryMessage, JobOptions};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, factory};
use tokio::sync::oneshot;
use tracing::{info, trace, warn};

use crate::actors::{RunnerMsg, StateMsg};
use crate::{EngineStatus, Job, JobState, SchedulerError, WaitGroup, Work};

/// Messages that can be sent to control the scheduler's operation
#[derive(Debug)]
pub enum SchedulerMsg {
    // Basic operations
    /// Starts the timers or resumes a suspended scheduler.
    Start,
    /// Pauses dispatch while jobs already handed to the factory keep running.
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
    #[default]
    Idle,
    Starting,
    Suspended,
}

/// Configuration arguments required to initialize the scheduler actor
#[derive(Debug)]
pub struct SchedulerArguments {
    pub sync: Duration,
    pub tick: Duration,
    pub last: usize,
    /// Number of runner slots managed by this scheduler.
    pub workers: usize,

    // Actors
    pub state: ActorRef<StateMsg>,
    pub factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
}

/// Internal state maintained by the scheduler actor
#[derive(Debug)]
pub struct SchedulerState {
    mode: Mode,
    sync: Duration,
    tick: Duration,
    last: usize,
    workers: usize,

    // Actors
    state: ActorRef<StateMsg>,
    factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,

    // The queues
    waiting: VecDeque<Work>,
    running: BTreeSet<usize>,
}

impl SchedulerState {
    /// Fills available worker slots from the waiting queue.
    ///
    /// Each successful dispatch starts a completion watcher outside the mailbox.
    /// Both a runner reply and a dropped reply port release the tracked slot.
    /// If the factory has stopped accepting messages, dropping queued work delivers
    /// cancellation to its waiters.
    fn dispatch_ready(&mut self, scheduler: &ActorRef<SchedulerMsg>) {
        if self.mode != Mode::Idle {
            return;
        }
        while self.running.len() < self.workers {
            let Some(work) = self.waiting.pop_front() else {
                break;
            };
            let id = work.id();
            let (reply, finished) = oneshot::channel();
            let dispatch = FactoryMessage::Dispatch(factory::Job {
                key: id,
                msg: RunnerMsg::Run(work, reply.into()),
                options: JobOptions::default(),
                accepted: None,
            });
            if self.factory.cast(dispatch).is_err() {
                warn!(job_id = id, "Runner factory closed during dispatch");
                self.waiting.clear();
                break;
            }

            self.running.insert(id);
            let scheduler = scheduler.clone();
            let cell = scheduler.get_cell();
            tokio::spawn(async move {
                tokio::select! {
                    _ = finished => {
                        let _ = scheduler.cast(SchedulerMsg::Finished(id));
                    }
                    _ = cell.wait(None) => {}
                }
            });
        }
    }

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
            workers: args.workers,
            state: args.state.clone(),
            factory: args.factory.clone(),
            waiting: VecDeque::new(),
            running: BTreeSet::new(),
        })
    }

    /// Handles scheduler commands without awaiting factory work and applies queue
    /// changes by job ID.
    #[tracing::instrument(skip(self, myself, message, state))]
    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            // ----- Basic operations
            SchedulerMsg::Tick => {
                state.dispatch_ready(&myself);
            }
            // sync is handled here now.
            //
            SchedulerMsg::Sync => {
                let _ = state.state.cast(StateMsg::Sync);
            }
            // The real start of the scheduler, we will generate clock ticks now.
            //
            SchedulerMsg::Start => {
                if state.mode == Mode::Starting {
                    // Begin with a sync.
                    //
                    let _ = state.state.cast(StateMsg::Sync);

                    // Set our clocks up
                    //
                    myself.send_interval(state.tick, || SchedulerMsg::Tick);
                    myself.send_interval(state.sync, || SchedulerMsg::Sync);
                }
                state.mode = Mode::Idle;
                state.dispatch_ready(&myself);
            }
            // Suspend if idle
            //
            SchedulerMsg::Suspend => {
                // Idle -> Suspend
                //
                if state.mode == Mode::Idle {
                    state.mode = Mode::Suspended;
                }
            }
            // Shutdown
            //
            SchedulerMsg::Stop => {
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
                state.dispatch_ready(&myself);
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
    use crate::actors::{
        ResultsActor, ResultsMsg, RunnerActor, RunnerArgs, RunnerBuilder, StatsActor,
    };
    use crate::{Consumer, Producer, Read, Save};
    use crate::{JobOutcome, JobStatus, Stats};
    use fetiche_common::Container;
    use fetiche_formats::Format;
    use ractor::factory::{Factory, FactoryArguments, queues, routing};
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

    /// Starts a scheduler with a test state actor and timers long enough for manual ticks.
    async fn spawn_scheduler(
        factory: ActorRef<FactoryMessage<usize, RunnerMsg>>,
        workers: usize,
    ) -> (ActorRef<SchedulerMsg>, ActorRef<StateMsg>) {
        let (state_tx, _state_rx) = mpsc::unbounded_channel();
        let (state, _) = Actor::spawn(None, Forward(std::marker::PhantomData), state_tx)
            .await
            .unwrap();
        let (scheduler, _) = Actor::spawn(
            None,
            SchedulerActor,
            SchedulerArguments {
                sync: Duration::from_secs(3600),
                tick: Duration::from_secs(3600),
                last: 1,
                workers,
                state: state.clone(),
                factory,
            },
        )
        .await
        .unwrap();
        (scheduler, state)
    }

    impl Fixture {
        /// Starts a scheduler with manual ticks and a controllable factory.
        async fn new(workers: usize) -> Self {
            let (factory_tx, dispatched) = mpsc::unbounded_channel();
            let (factory, _) = Actor::spawn(None, Forward(std::marker::PhantomData), factory_tx)
                .await
                .unwrap();
            let (scheduler, state) = spawn_scheduler(factory.clone(), workers).await;
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

        /// Retrieves a dispatched job and verifies the factory and work IDs agree.
        async fn next_dispatch(&mut self, expected: usize) -> (Work, RpcReplyPort<()>) {
            let message = timeout(TEST_TIMEOUT, self.dispatched.recv())
                .await
                .expect("scheduler did not dispatch work")
                .unwrap();
            let FactoryMessage::Dispatch(job) = message else {
                panic!("expected a dispatch");
            };
            let RunnerMsg::Run(work, reply) = job.msg;
            assert_eq!(job.key, expected);
            assert_eq!(work.id(), expected);
            (work, reply)
        }

        /// Waits for the completion watcher to reconcile the expected running IDs.
        async fn wait_for_running(&self, expected: &[usize]) {
            timeout(TEST_TIMEOUT, async {
                loop {
                    if call!(self.scheduler, SchedulerMsg::List).unwrap() == expected {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("running IDs did not settle");
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
        let mut fixture = Fixture::new(1).await;
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

    /// Keeps the mailbox responsive while both slots are occupied and refills the
    /// slot of a later job that finishes first.
    #[tokio::test]
    async fn dispatch_refills_the_correct_slot_after_out_of_order_completion() {
        let mut fixture = Fixture::new(2).await;
        let _first = fixture.enqueue(40).await.unwrap();
        let second = fixture.enqueue(90).await.unwrap();
        let _third = fixture.enqueue(110).await.unwrap();
        fixture.scheduler.cast(SchedulerMsg::Start).unwrap();
        let (_first_work, _first_reply) = fixture.next_dispatch(40).await;
        let (second_work, second_reply) = fixture.next_dispatch(90).await;

        fixture.scheduler.cast(SchedulerMsg::Tick).unwrap();
        assert_eq!(
            call!(fixture.scheduler, SchedulerMsg::List).unwrap(),
            [40, 90]
        );
        assert!(fixture.dispatched.try_recv().is_err());
        assert_eq!(fixture.enqueue(200).await.unwrap().id, 200);
        assert!(matches!(
            call!(fixture.scheduler, |port| SchedulerMsg::RemoveById(90, port)).unwrap(),
            Err(EngineStatus::JobIsRunning(90))
        ));

        second_work.completion.send(JobOutcome {
            id: 90,
            status: JobStatus::Succeeded,
            stats: Stats::default(),
        });
        second_reply.send(()).unwrap();
        let (_third_work, _third_reply) = fixture.next_dispatch(110).await;
        assert_eq!(
            call!(fixture.scheduler, SchedulerMsg::List).unwrap(),
            [40, 110]
        );
        assert_eq!(second.rx.await.unwrap().status, JobStatus::Succeeded);
    }

    /// Processes a dropped factory job while suspended, then dispatches the next
    /// job only after the scheduler resumes.
    #[tokio::test]
    async fn suspension_keeps_control_and_completion_messages_responsive() {
        let mut fixture = Fixture::new(1).await;
        let first = fixture.enqueue(40).await.unwrap();
        fixture.scheduler.cast(SchedulerMsg::Start).unwrap();
        let (work, reply) = fixture.next_dispatch(40).await;
        fixture.scheduler.cast(SchedulerMsg::Suspend).unwrap();
        let _second = fixture.enqueue(90).await.unwrap();
        fixture.scheduler.cast(SchedulerMsg::Tick).unwrap();
        assert_eq!(call!(fixture.scheduler, SchedulerMsg::List).unwrap(), [40]);

        drop(work);
        drop(reply);
        let outcome = timeout(TEST_TIMEOUT, first.rx).await.unwrap().unwrap();
        assert_eq!(outcome.status, JobStatus::Cancelled { stage: None });
        fixture.wait_for_running(&[]).await;
        assert!(!call!(fixture.scheduler, SchedulerMsg::Empty).unwrap());
        assert!(fixture.dispatched.try_recv().is_err());

        fixture.scheduler.cast(SchedulerMsg::Start).unwrap();
        let (_work, _reply) = fixture.next_dispatch(90).await;
    }

    /// A stopped factory cancels waiting jobs instead of leaving them outstanding.
    #[tokio::test]
    async fn closed_factory_cancels_waiting_jobs() {
        let fixture = Fixture::new(2).await;
        let first = fixture.enqueue(40).await.unwrap();
        let second = fixture.enqueue(90).await.unwrap();
        fixture
            .factory
            .get_cell()
            .kill_and_wait(Some(TEST_TIMEOUT))
            .await
            .unwrap();
        fixture.scheduler.cast(SchedulerMsg::Start).unwrap();

        for waiter in [first, second] {
            let outcome = timeout(TEST_TIMEOUT, waiter.rx).await.unwrap().unwrap();
            assert_eq!(outcome.status, JobStatus::Cancelled { stage: None });
        }
        assert!(call!(fixture.scheduler, SchedulerMsg::Empty).unwrap());
    }

    /// Runs a local file job through actual runners and verifies its published outcome.
    #[tokio::test]
    async fn scheduler_runs_and_retires_a_local_file_job() {
        let (stats, _) = Actor::spawn(
            Some("scheduler-test-stats".into()),
            StatsActor,
            "test".into(),
        )
        .await
        .unwrap();
        let (results, _) = Actor::spawn(None, ResultsActor, ()).await.unwrap();
        let factory = Factory::<
            usize,
            RunnerMsg,
            RunnerArgs,
            RunnerActor,
            routing::QueuerRouting<usize, RunnerMsg>,
            queues::DefaultQueue<usize, RunnerMsg>,
        >::default();
        let args = FactoryArguments::builder()
            .worker_builder(Box::new(RunnerBuilder {
                queue_capacity: 1,
                results: results.clone(),
                stat: stats.clone(),
            }))
            .queue(Default::default())
            .router(Default::default())
            .num_initial_workers(1)
            .build();
        let (factory, _) = Actor::spawn(None, factory, args).await.unwrap();
        let (scheduler, state) = spawn_scheduler(factory.clone(), 1).await;

        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.txt");
        let output = dir.path().join("output.txt");
        tokio::fs::write(&input, "scheduled data").await.unwrap();
        let mut read = Read::new("local input");
        read.path(input.to_str().unwrap());
        let mut save = Save::new("local output", Format::None, Container::Raw);
        save.path(output.to_str().unwrap());
        let job = Job::builder()
            .id(17)
            .state(JobState::Ready)
            .producer(Producer::Read(read))
            .consumer(Consumer::Save(save))
            .build();
        let waiter = call!(scheduler, |port| SchedulerMsg::Add(job, port))
            .unwrap()
            .unwrap();
        scheduler.cast(SchedulerMsg::Start).unwrap();

        let outcome = timeout(TEST_TIMEOUT, waiter.rx).await.unwrap().unwrap();
        assert_eq!(outcome.status, JobStatus::Succeeded);
        assert_eq!(
            tokio::fs::read_to_string(output).await.unwrap(),
            "scheduled data"
        );
        assert_eq!(
            call!(results, |port| ResultsMsg::Fetch(17, port)).unwrap(),
            Some(outcome)
        );
        timeout(TEST_TIMEOUT, async {
            while !call!(scheduler, SchedulerMsg::Empty).unwrap() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("completed job did not leave the running queue");

        scheduler.kill();
        factory.stop(None);
        state.stop(None);
        stats.stop(None);
        results.stop(None);
    }
}
