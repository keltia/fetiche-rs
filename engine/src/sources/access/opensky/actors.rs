use std::fmt::Debug;

use ractor::{Actor, ActorProcessingErr, ActorRef};

use crate::actors::StatsMsg;

#[derive(Debug)]
pub enum OpenskyMsg {
    Consume(String),
}

#[ractor::async_trait]
impl Actor for Opensky {
    type Message = StatsMsg;
    type State = ();
    type Result = ();

    fn handle(&mut self, msg: Self::Message, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        todo!()
    }
}
