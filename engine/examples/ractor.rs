// Tokio-based worker/alarm threads
//
// We may be going full async, hang on Baby, we're for a ride!
//

use std::time::Duration;

use eyre::Result;
use ractor::{async_trait, pg, registry, Actor, ActorProcessingErr, ActorRef};
use tokio::time::sleep;
use tracing::trace;

const PG_NAME: &str = "workers";
#[derive(Debug)]
struct Worker;

enum WorkerMsg {
    Tick,
    Change(String),
}

#[async_trait]
impl Actor for Worker {
    type Msg = WorkerMsg;
    type State = String;
    type Arguments = String;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        eprintln!("pre_start for {args}");

        pg::join(PG_NAME.into(), vec![myself.get_cell()]);
        eprintln!("{args}");
        Ok(args)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        let chr = String::from(state.chars().nth(0).unwrap());
        match message {
            WorkerMsg::Tick => {
                state.push_str(&chr);
                eprintln!("{state}");
            }
            WorkerMsg::Change(c) => {
                state.push_str(&c);
                eprintln!("character changed to {c}");
            }
        }
        Ok(())
    }
}

// If 0, infinite wait, need SIGINT to sop
//
const SLEEP: Duration = Duration::from_secs(20);
const WAIT: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() -> Result<()> {
    let (w1, h1) = Actor::spawn(Some("r1".to_string()), Worker, ".".into()).await?;
    w1.send_interval(WAIT, || WorkerMsg::Tick);
    w1.exit_after(SLEEP);

    let (w2, h2) = Actor::spawn(Some("r2".to_string()), Worker, "+".into()).await?;
    w2.send_interval(WAIT, || WorkerMsg::Tick);
    w2.exit_after(SLEEP);

    let list = registry::registered();
    eprintln!("Currently registered actors:");
    list.iter().for_each(|actor| {
        eprintln!("  {actor}");
    });

    let wt1 = w1.clone();
    tokio::spawn(async move {
        sleep(Duration::from_secs(7u64)).await;
        wt1.cast(WorkerMsg::Change("*".into()))
    })
        .await?
        .expect("TODO: panic message");

    tokio::spawn(async move {
        let workers = String::from(PG_NAME);

        let _ = ctrlc::set_handler(move || {
            trace!("Ctrl-C pressed");
            pg::get_members(&workers).iter().for_each(|cell| {
                cell.stop(Some("ctrl-C pressed".into()));
            });
        });
    })
        .await?;

    h1.await?;
    h2.await?;
    let list = registry::registered();
    assert!(list.is_empty());

    eprintln!("No more actors");
    Ok(())
}
