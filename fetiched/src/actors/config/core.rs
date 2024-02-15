use std::str::FromStr;

use actix::dev::{MessageResponse, OneshotSender};
use actix::{Actor, Message};
use serde::Serialize;
use strum::EnumString;

#[derive(Clone, Debug, strum::Display, EnumString, strum::VariantNames, Serialize)]
pub enum Param {
    Integer(i32),
    String(String),
}

impl FromStr for Param {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Param::String(s.to_owned()))
    }
}

impl From<i32> for Param {
    fn from(value: i32) -> Self {
        Param::Integer(value)
    }
}

impl From<u32> for Param {
    fn from(value: u32) -> Self {
        Param::Integer(value as i32)
    }
}

impl<A, M> MessageResponse<A, M> for Param
where
    A: Actor,
    M: Message<Result = Param>,
{
    fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            let _ = tx.send(self);
        }
    }
}
