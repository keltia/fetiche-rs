use eyre::Result;
use fetiche_common::{ConfigFile, IntoConfig, Versioned};
use fetiche_macros::into_configfile;
use ractor::registry::registered;
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Deserialize, Serialize)]
struct ConfigActor;

#[into_configfile]
#[derive(Debug, Default, Deserialize, Serialize)]
struct Foo {
    a: String,
    b: usize,
}

#[async_trait]
impl Actor for ConfigActor {
    type Msg = ();
    type State = ConfigFile<Foo>;
    type Arguments = String;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        let cfg = ConfigFile::<Foo>::load(Some(&args))?;
        Ok(cfg)
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        _message: Self::Msg,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (_w, _h) = Actor::spawn(
        Some(String::from("configactor")),
        ConfigActor,
        "local.hcl".to_string(),
    )
        .await?;

    let list = registered();
    dbg!(&list);

    Ok(())
}
