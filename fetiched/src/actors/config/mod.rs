//! This an `Actor` implementing a basic get/set key/value store for configuration variables.
//!
//! API:
//!
//! - `ConfigGet`
//! - `ConfigSet`
//! - `ConfigList`
//! - `ConfigKeys`
//!

use std::collections::HashMap;
use std::str::FromStr;

use actix::dev::MessageResponse;
use actix::{Actor, Context, Handler, Message};
use eyre::Result;
use serde::Serialize;
use tracing::{info, trace};

pub use core::*;

mod core;

// ----- Messages

/// Get a single parameter
///
#[derive(Debug, Message)]
#[rtype(result = "Result<Param>")]
pub struct ConfigGet {
    pub name: String,
}

/// Set a single parameter
///
#[derive(Debug, Message)]
#[rtype(result = "Result<()>")]
pub struct ConfigSet {
    pub name: String,
    pub value: Param,
}

/// Get a json dump of all parameters
///
#[derive(Debug, Message)]
#[rtype(result = "Result<String>")]
pub struct ConfigList;

/// Get all keys
///
#[derive(Debug, Message)]
#[rtype(result = "Result<Vec<String>>")]
pub struct ConfigKeys;

// ----- The Actor

#[derive(Debug)]
pub struct ConfigActor {
    config: HashMap<String, Param>,
}

impl Default for ConfigActor {
    fn default() -> Self {
        let mut h = HashMap::<String, Param>::new();

        h.insert("version".to_string(), Param::Integer(1));

        Self { config: h.into() }
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
    type Result = Result<Param>;

    fn handle(&mut self, msg: ConfigGet, _: &mut Self::Context) -> Self::Result {
        trace!("config::get");

        let res = match self.config.get(&msg.name) {
            Some(res) => res,
            None => return Err(eyre::eyre!("Unknown parameter {}", &msg.name)),
        };
        Ok(res.clone())
    }
}

impl Handler<ConfigSet> for ConfigActor {
    type Result = Result<()>;

    fn handle(&mut self, msg: ConfigSet, _: &mut Self::Context) -> Self::Result {
        trace!("config::set");

        self.config.insert(msg.name, msg.value);
        Ok(())
    }
}

impl Handler<ConfigList> for ConfigActor {
    type Result = Result<String>;

    fn handle(&mut self, msg: ConfigList, _: &mut Self::Context) -> Self::Result {
        trace!("config::list");

        Ok(serde_json::to_string(&self.config)?)
    }
}

impl Handler<ConfigKeys> for ConfigActor {
    type Result = Result<Vec<String>>;

    fn handle(&mut self, msg: ConfigKeys, ctx: &mut Self::Context) -> Self::Result {
        trace!("config::keys");

        let keys: Vec<_> = self.config.keys().map(|k| k.to_owned()).collect();
        Ok(keys)
    }
}
