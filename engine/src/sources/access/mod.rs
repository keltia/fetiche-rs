pub use error::*;

/// Stops a source actor when its owning stream exits or is cancelled.
#[cfg(any(feature = "avionix", feature = "senhive"))]
pub(super) struct KillActorOnDrop(pub ractor::ActorCell);

#[cfg(any(feature = "avionix", feature = "senhive"))]
impl Drop for KillActorOnDrop {
    /// Sends the actor a kill signal during stream cleanup.
    fn drop(&mut self) {
        self.0.kill();
    }
}

/// Waits for the stream deadline, then signals the worker to stop and joins it.
///
/// A zero duration disables the deadline. A worker that exits before the
/// deadline is an error, and a failed task join is propagated to the caller.
/// The caller owns a [`KillActorOnDrop`] guard to cover cancellation of this wait.
#[cfg(any(feature = "avionix", feature = "senhive"))]
async fn wait_for_stream(
    worker: ractor::ActorCell,
    mut handle: tokio::task::JoinHandle<()>,
    duration: std::time::Duration,
) -> eyre::Result<()> {
    let deadline = async {
        if duration.is_zero() {
            std::future::pending::<()>().await;
        }
        tokio::time::sleep(duration).await;
    };
    tokio::select! {
        result = &mut handle => {
            result?;
            Err(eyre::eyre!("source worker stopped before the stream ended"))
        }
        () = deadline => {
            worker.kill();
            handle.await?;
            Ok(())
        }
    }
}

#[cfg(feature = "aeroscope")]
pub use aeroscope::*;
#[cfg(feature = "asd")]
pub use asd::*;
#[cfg(feature = "avionix")]
pub use avionix::*;
#[cfg(feature = "flightaware")]
pub use flightaware::*;
#[cfg(feature = "opensky")]
pub use opensky::*;
#[cfg(feature = "safesky")]
pub use safesky::*;
#[cfg(feature = "senhive")]
pub use senhive::*;

mod error;

#[cfg(feature = "aeroscope")]
mod aeroscope;
#[cfg(feature = "asd")]
mod asd;
#[cfg(feature = "avionix")]
mod avionix;
#[cfg(feature = "flightaware")]
mod flightaware;
#[cfg(feature = "opensky")]
mod opensky;
#[cfg(feature = "safesky")]
mod safesky;
#[cfg(feature = "senhive")]
mod senhive;
