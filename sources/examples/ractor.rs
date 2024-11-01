// Tokio-based worker/alarm threads
//
// We may be going full async, hang on Baby, we're for a ride!
//

use std::time::Duration;

use eyre::Result;
use ractor::{async_trait, pg, Actor, ActorProcessingErr, ActorRef};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tokio::time::sleep;

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
        myself: ActorRef<Self::Msg>,
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

struct Signals;

#[async_trait]
impl Actor for Signals {
    type Msg = ();
    type State = ();
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        eprintln!("Signals starting...");
        Ok(())
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        eprintln!("Setting up signal handling...");

        // setup ctrl-c handled
        //
        #[cfg(windows)]
        let mut sig = ctrl_c()?;

        #[cfg(unix)]
        let mut stream = signal(SignalKind::interrupt())?;

        let workers = String::from(PG_NAME);
        #[cfg(windows)]
        tokio::select! {
            _ = sig.recv() => {
                pg::get_members(&workers).iter().for_each(|cell| {
                    cell.stop(Some("ctrl-C pressed".into()));
                })
            },
            else => ()
        }

        #[cfg(unix)]
        tokio::select! {
            Some(_) = stream.recv() => {
                eprintln!("Got SIGINT");
                pg::get_members(&workers).iter().for_each(|cell| {
                    cell.stop(Some("ctrl-C pressed".into()));
                });
                myself.kill()
            },
            else => ()
        }

        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: Self::Msg,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        Ok(())
    }
}
// If 0, infinite wait, need SIGINT to sop
//
const SLEEP: Duration = Duration::from_secs(20);
const WAIT: Duration = Duration::from_secs(2);

#[tokio::main]
async fn main() -> Result<()> {
    let (ws, _hs) = Actor::spawn(Some("sigs".into()), Signals, ()).await?;

    let (w1, h1) = Actor::spawn(Some("r1".to_string()), Worker, ".".into()).await?;
    w1.send_interval(WAIT, || WorkerMsg::Tick);
    w1.exit_after(SLEEP);

    let (w2, h2) = Actor::spawn(Some("r2".to_string()), Worker, "+".into()).await?;
    w2.send_interval(WAIT, || WorkerMsg::Tick);
    w2.exit_after(SLEEP);

    sleep(Duration::from_secs(7u64)).await;
    let _ = w1.cast(WorkerMsg::Change("*".into()));

    ws.kill();
    h1.await?;
    h2.await?;
    eprintln!("with sleeper, nothing is displayed");
    Ok(())
}
