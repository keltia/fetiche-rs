//! This an `Actor` implementing a basic get/set key/value store for configuration variables.
//!

use std::collections::HashMap;

use actix::dev::{MessageResponse, OneshotSender};
use actix::{Actor, Context, Handler, Message};
use serde::Serialize;
use strum::{EnumString, EnumVariantNames};
use tracing::{info, trace};

// -----

#[derive(Clone, Debug, strum::Display, EnumString, EnumVariantNames, Serialize)]
pub enum Param {
    Integer(i32),
    String(String),
}

impl<A, M> MessageResponse<A, M> for Param
where
    A: Actor,
    M: Message<Result = Param>,
{
    fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

// -----

#[derive(Debug)]
pub struct ConfigGet {
    pub name: String,
}

impl Message for ConfigGet {
    type Result = Param;
}

#[derive(Debug)]
pub struct ConfigSet {
    pub name: String,
    pub value: Param,
}

impl Message for ConfigSet {
    type Result = eyre::Result<()>;
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct ConfigList {}

#[derive(Debug)]
pub struct ConfigActor {
    c: HashMap<String, Param>,
}

impl Default for ConfigActor {
    fn default() -> Self {
        let mut h = HashMap::<String, Param>::new();

        h.insert("version".to_string(), Param::Integer(1));

        Self { c: h.into() }
    }
}

impl Actor for ConfigActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        info!("Config is alive");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        info!("Config is stopped");
    }
}

impl Handler<ConfigGet> for ConfigActor {
    type Result = Param;

    fn handle(&mut self, msg: ConfigGet, _: &mut Self::Context) -> Self::Result {
        trace!("config::get");
        self.c.get(&msg.name).unwrap().clone()
    }
}

impl Handler<ConfigSet> for ConfigActor {
    type Result = eyre::Result<()>;

    fn handle(&mut self, msg: ConfigSet, _: &mut Self::Context) -> Self::Result {
        trace!("config::set");
        self.c.insert(msg.name, msg.value);
        Ok(())
    }
}

impl Handler<ConfigList> for ConfigActor {
    type Result = ();

    fn handle(&mut self, msg: ConfigList, _: &mut Self::Context) -> Self::Result {
        trace!("config::list");

        info!("{}", serde_json::to_string(&self.c).unwrap());
    }
}
