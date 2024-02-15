use actix::Addr;
use strum::EnumString;

pub use config::*;
pub use engine::*;
use fetiche_sources::makepath;
pub use state::*;
pub use storage::*;

mod config;
mod engine;
mod state;
mod storage;

/// This is a "bus" that regroup all actors' address for communication
///
#[derive(Clone, Debug)]
pub struct Bus {
    /// K/V configuration agent
    pub config: Addr<ConfigActor>,
    /// State management agent
    pub state: Addr<StateActor>,
    /// Storage management agent
    pub store: Addr<StorageActor>,
}

/// Current registered sub-systems
///
#[derive(Debug, EnumString, strum::Display, strum::VariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum System {
    Config,
    Engine,
    State,
    Storage,
}

/// Macro to generate boilerplate code for non-builtin structs.
///
#[macro_export]
macro_rules! response_for {
    ($struct:ident) => {
        impl<A, M> MessageResponse<A, M> for $struct
        where
            A: Actor,
            M: Message<Result = $struct>,
        {
            fn handle(self, _ctx: &mut A::Context, tx: Option<OneshotSender<M::Result>>) {
                if let Some(tx) = tx {
                    let _ = tx.send(self);
                }
            }
        }
    };
}
