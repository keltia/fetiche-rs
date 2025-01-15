//! Example of an actor-wrapped Fetiche engine.
//!

use eyre::Result;
use fetiche_engine::Engine;
use ractor::{async_trait, call, Actor, ActorProcessingErr, ActorRef, RpcReplyPort};

struct EngineActor;

struct EngineState {
    pub e: Engine,
}

#[derive(Debug)]
enum EngineMsg {
    CreateJob(String, RpcReplyPort<usize>),
    RemoveJob(usize),
    Sources(RpcReplyPort<usize>),
    Version(RpcReplyPort<String>),
}

#[async_trait]
impl Actor for EngineActor {
    type Msg = EngineMsg;
    type State = EngineState;
    type Arguments = ();

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        _args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        let e = Engine::new();
        Ok(EngineState { e })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match message {
            EngineMsg::CreateJob(name, sender) => {
                let job = state.e.create_job(&name);
                let _ = sender.send(job.id);
            }
            EngineMsg::RemoveJob(id) => {
                let _ = state.e.remove_job(id);
            }
            EngineMsg::Version(sender) => {
                let _ = sender.send(state.e.version());
            }
            EngineMsg::Sources(sender) => {
                let srcs = state.e.sources();
                let _ = sender.send(srcs.len());
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (engine, _h) = Actor::spawn(Some("engine".to_string()), EngineActor, ()).await?;

    let resp = call!(engine, |port| EngineMsg::Version(port))?;
    let n = call!(engine, |port| EngineMsg::Sources(port))?;

    println!("{:?}", resp);
    println!("# of sources {:?}", n);
    Ok(())
}
