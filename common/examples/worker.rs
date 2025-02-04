use ractor::factory::*;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_tree::HierarchicalLayer;

#[derive(Debug)]
enum ExampleMessage {
    PrintValue(u64),
    EchoValue(u64, RpcReplyPort<u64>),
}

/// The worker's specification for the factory. This defines
/// the business logic for each message that will be done in parallel.
struct ExampleWorker;

#[ractor::async_trait]
impl Worker for ExampleWorker {
    type Key = ();
    type Message = ExampleMessage;
    type Arguments = ();
    type State = ();
    async fn pre_start(
        &self,
        _wid: WorkerId,
        _factory: &ActorRef<FactoryMessage<(), ExampleMessage>>,
        startup_context: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        Ok(startup_context)
    }
    async fn handle(
        &self,
        wid: WorkerId,
        _factory: &ActorRef<FactoryMessage<(), ExampleMessage>>,
        Job { msg, key, .. }: Job<(), ExampleMessage>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        // Actual business logic that we want to parallelize
        tracing::trace!("Worker {} received {:?}", wid, msg);
        match msg {
            ExampleMessage::PrintValue(value) => {
                tracing::info!("Worker {} printing value {value}", wid);
            }
            ExampleMessage::EchoValue(value, reply) => {
                tracing::info!("Worker {} echoing value {value}", wid);
                let _ = reply.send(value);
            }
        }
        Ok(key)
    }
}
/// Used by the factory to build new [ExampleWorker]s.
struct ExampleWorkerBuilder;
impl WorkerBuilder<ExampleWorker, ()> for ExampleWorkerBuilder {
    fn build(&mut self, _wid: usize) -> (ExampleWorker, ()) {
        (ExampleWorker, ())
    }
}
#[tokio::main]
async fn main() {
    // Initialise logging early
    //
    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Do we want hierarchical output?
    //
    let tree = Some(
        HierarchicalLayer::new(2)
            .with_ansi(true)
            .with_span_retrace(true)
            .with_span_modes(true)
            .with_targets(true)
            .with_verbose_entry(true)
            .with_verbose_exit(true)
            .with_bracketed_fields(true),
    );

    // Combine filters & exporters
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .init();

    let factory_def = Factory::<
        (),
        ExampleMessage,
        (),
        ExampleWorker,
        routing::QueuerRouting<(), ExampleMessage>,
        queues::DefaultQueue<(), ExampleMessage>
    >::default();
    let factory_args = FactoryArguments::builder()
        .worker_builder(Box::new(ExampleWorkerBuilder))
        .queue(Default::default())
        .router(Default::default())
        .num_initial_workers(5)
        .build();

    let (factory, handle) = Actor::spawn(None, factory_def, factory_args)
        .await
        .expect("Failed to startup factory");
    for i in 0..99 {
        factory
            .cast(FactoryMessage::Dispatch(Job {
                key: (),
                msg: ExampleMessage::PrintValue(i),
                options: JobOptions::default(),
                accepted: None,
            }))
            .expect("Failed to send to factory");
    }
    let reply = factory
        .call(
            |prt| {
                FactoryMessage::Dispatch(Job {
                    key: (),
                    msg: ExampleMessage::EchoValue(123, prt),
                    options: JobOptions::default(),
                    accepted: None,
                })
            },
            None,
        )
        .await
        .expect("Failed to send to factory")
        .expect("Failed to parse reply");
    assert_eq!(reply, 123);
    factory.stop(None);
    handle.await.unwrap();
}
