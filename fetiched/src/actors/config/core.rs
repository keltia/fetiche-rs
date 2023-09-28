use std::str::FromStr;

use actix::dev::{MessageResponse, OneshotSender};
use actix::{Actor, Addr, Message};
use serde::Serialize;
use strum::EnumVariantNames;

#[derive(Clone, Debug, strum::Display, EnumVariantNames, Serialize)]
pub enum Param<A>
where
    A: Actor,
{
    Addr(Addr<A>),
    Integer(i32),
    String(String),
}

impl<T> FromStr for Param<T> {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Param::String(s.to_owned()))
    }
}

impl<T> From<i32> for Param<T> {
    fn from(value: i32) -> Self {
        Param::Integer(value)
    }
}

impl<T> From<u32> for Param<T> {
    fn from(value: u32) -> Self {
        Param::Integer(value as i32)
    }
}

impl<A> From<Addr<A>> for Param<A>
where
    A: Actor,
{
    fn from(value: Addr<A>) -> Self {
        Param::Addr(value)
    }
}

impl<A, M, T> MessageResponse<A, M> for Param<T>
where
    A: Actor,
    M: Message<Result = Param<T>>,
{
    fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
        if let Some(tx) = tx {
            let _ = tx.send(self);
        }
    }
}
