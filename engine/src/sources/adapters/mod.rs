//! Provider-specific network adapters and their shared stream cleanup.

pub use error::{AccessError, DataError, ParamError};

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

mod error;

// The persisted format is also supported when the ASD adapter is disabled.
#[path = "asd/token.rs"]
pub(crate) mod asd_token;

#[cfg(feature = "aeroscope")]
pub(crate) mod aeroscope;
#[cfg(feature = "asd")]
pub(crate) mod asd;
#[cfg(feature = "avionix")]
pub(crate) mod avionix;
#[cfg(feature = "flightaware")]
pub(crate) mod flightaware;
#[cfg(feature = "opensky")]
pub(crate) mod opensky;
#[cfg(feature = "safesky")]
pub(crate) mod safesky;
#[cfg(feature = "senhive")]
pub(crate) mod senhive;

#[cfg(all(test, any(feature = "avionix", feature = "senhive")))]
mod tests {
    use super::*;
    use ractor::{Actor, ActorProcessingErr, ActorRef};
    use std::time::Duration;
    use tokio::time::timeout;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    struct IdleWorker;

    #[ractor::async_trait]
    impl Actor for IdleWorker {
        type Msg = ();
        type State = ();
        type Arguments = ();

        /// Starts the idle actor used to exercise stream cleanup without a network source.
        async fn pre_start(
            &self,
            _myself: ActorRef<Self::Msg>,
            _args: Self::Arguments,
        ) -> Result<Self::State, ActorProcessingErr> {
            Ok(())
        }
    }

    /// An expired stream deadline stops the worker and waits for its exit.
    #[tokio::test]
    async fn stream_deadline_stops_the_worker() {
        let (worker, handle) = Actor::spawn(None, IdleWorker, ()).await.unwrap();
        let _guard = KillActorOnDrop(worker.get_cell());
        timeout(
            TEST_TIMEOUT,
            wait_for_stream(worker.get_cell(), handle, Duration::from_millis(1)),
        )
        .await
        .expect("stream deadline did not stop the worker")
        .unwrap();
        assert_eq!(worker.get_cell().get_status(), ractor::ActorStatus::Stopped);
    }

    /// Even an indefinite stream reports a worker that has already stopped.
    #[tokio::test]
    async fn indefinite_stream_reports_worker_exit() {
        let (worker, handle) = Actor::spawn(None, IdleWorker, ()).await.unwrap();
        worker.stop(None);
        let error = timeout(
            TEST_TIMEOUT,
            wait_for_stream(worker.get_cell(), handle, Duration::ZERO),
        )
        .await
        .expect("worker exit did not complete the stream wait")
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            "source worker stopped before the stream ended"
        );
    }

    /// Dropping the stream's guard stops its actor even outside the deadline path.
    #[tokio::test]
    async fn dropping_stream_guard_stops_the_worker() {
        let (worker, handle) = Actor::spawn(None, IdleWorker, ()).await.unwrap();
        drop(KillActorOnDrop(worker.get_cell()));
        timeout(TEST_TIMEOUT, handle)
            .await
            .expect("dropping the guard did not stop the worker")
            .unwrap();
    }
}
