//! Feeder for AMQP topics.
//!
//! URL: amqp://USER:PASSWORD@HOST:PORT/topic
//!
use crate::{Consumer, Runnable};
use fetiche_formats::Format;
use std::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;

use fetiche_macros::RunnableDerive;

#[derive(Clone, Debug, RunnableDerive, PartialEq)]
pub struct Feeder {
    /// URL to the broker.
    broker: String,
    /// Name of the feeding topic.
    topic: String,
    /// Payload format.  This needed for deserialisation.
    payload: Format,
}

impl Feeder {
    pub fn new(url: &str, payload: Format) -> Self {
        let (broker, topic) = url.split('/').to_owned().collect();
        Feeder {
            broker: broker.to_string(),
            topic: topic.to_string(),
            payload,
        }
    }

    pub async fn execute(&mut self, data: String, _stdout: Sender<String>) -> eyre::Result<()> {
        todo!();
    }
}

impl From<Feeder> for Consumer {
    fn from(f: Feeder) -> Self {
        Consumer::Feeder(f)
    }
}

impl Runnable for Feeder {
    async fn run(&mut self, out: Receiver<String>) -> (Receiver<String>, JoinHandle<eyre::Result<()>>) {
        todo!()
    }
}

