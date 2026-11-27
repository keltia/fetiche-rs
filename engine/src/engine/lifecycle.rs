//! Ownership of the actors and the running marker for one engine instance.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use eyre::{Result, WrapErr};
use futures::future::{BoxFuture, FutureExt, Shared};
use ractor::{ActorCell, ActorRef, Message, call, cast};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::trace;

use crate::CANARY_FILE;
use crate::actors::{SchedulerMsg, StateMsg};

use super::Engine;

const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);
const FINALIZE_TIMEOUT: Duration = Duration::from_secs(5);
static NEXT_CLEANUP: AtomicUsize = AtomicUsize::new(0);

impl Engine {
    /// Synchronizes all engine state by persisting it to disk
    ///
    /// This method sends a sync message to the state actor which handles
    /// writing the current state to the configured state file location.
    ///
    #[tracing::instrument(skip(self))]
    #[inline]
    pub fn sync(&self) -> Result<()> {
        trace!("engine::sync");

        Ok(cast!(self.state, StateMsg::Sync)?)
    }

    /// Shuts down this engine with a five-second grace period for active jobs.
    ///
    /// See [`Self::shutdown_with_timeout`] for the shutdown sequence and errors.
    #[tracing::instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<()> {
        self.shutdown_with_timeout(SHUTDOWN_GRACE).await
    }

    /// Stops admission and gives dispatched jobs up to `grace` to finish.
    ///
    /// Queued jobs are cancelled immediately. After the grace period, active
    /// pipelines receive a cancellation signal. The engine flushes state and
    /// awaits its actors before removing its running marker. Calls on any
    /// clone share the same shutdown task and result. The first call sets the
    /// grace period.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot drain or cancel jobs, write final
    /// state, join the actors, or remove the running marker.
    ///
    /// ```no_run
    /// use fetiche_engine::Engine;
    /// use std::time::Duration;
    ///
    /// #[tokio::main]
    /// async fn main() -> eyre::Result<()> {
    ///     let engine = Engine::single().await?;
    ///     engine.shutdown_with_timeout(Duration::from_secs(2)).await?;
    ///     Ok(())
    /// }
    /// ```
    #[tracing::instrument(skip(self))]
    pub async fn shutdown_with_timeout(&self, grace: Duration) -> Result<()> {
        self.lifecycle
            .shutdown(self.scheduler.clone(), self.state.clone(), grace)
            .await
    }
}

type Shutdown = Shared<BoxFuture<'static, Result<(), String>>>;

/// Shared by all engine handles. The first shutdown request owns the cleanup task.
#[derive(Debug)]
pub(crate) struct Lifecycle {
    workspace: PathBuf,
    runtime: tokio::runtime::Handle,
    resources: Mutex<Option<Resources>>,
    shutdown: OnceLock<Shutdown>,
}

impl Lifecycle {
    /// Takes ownership of the resources accumulated during startup.
    pub(crate) fn new(resources: Resources, workspace: PathBuf) -> Self {
        Self {
            workspace,
            runtime: tokio::runtime::Handle::current(),
            resources: Mutex::new(Some(resources)),
            shutdown: OnceLock::new(),
        }
    }

    /// Returns the original workspace owned by this engine.
    pub(crate) fn workspace(&self) -> &Path {
        &self.workspace
    }

    /// Starts cleanup once and shares the result, even if a caller drops its wait.
    pub(crate) async fn shutdown(
        &self,
        scheduler: ActorRef<SchedulerMsg>,
        state: ActorRef<StateMsg>,
        grace: Duration,
    ) -> Result<()> {
        let task = self.shutdown.get_or_init(|| {
            let resources = self.resources.lock().unwrap().take().unwrap();
            self.runtime
                .spawn(async move { resources.shutdown(scheduler, state, grace).await })
                .map(|joined| {
                    joined
                        .map_err(|err| format!("engine shutdown task failed: {err}"))?
                        .map_err(|err| format!("{err:#}"))
                })
                .boxed()
                .shared()
        });
        task.clone().await.map_err(|err| eyre::eyre!(err))
    }
}

impl Drop for Lifecycle {
    /// On last-owner drop, cancels work and schedules best-effort actor cleanup.
    fn drop(&mut self) {
        if let Some(mut resources) = self.resources.get_mut().unwrap().take() {
            resources.cancel.send_replace(true);
            self.runtime.spawn(async move {
                let _ = resources.teardown().await;
            });
        }
    }
}

/// Tracks only resources created by this engine, including during partial startup.
#[derive(Debug)]
pub(crate) struct Resources {
    marker: Option<PathBuf>,
    actors: Vec<(ActorCell, JoinHandle<()>)>,
    cancel: watch::Sender<bool>,
}

impl Resources {
    /// Claims an existing workspace by creating its marker exclusively.
    pub(crate) fn claim(workspace: &Path) -> Result<Self> {
        let marker = workspace.join(CANARY_FILE);
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
            .wrap_err_with(|| format!("claim engine workspace {}", workspace.display()))?;
        Ok(Self {
            marker: Some(marker),
            actors: Vec::new(),
            cancel: watch::channel(false).0,
        })
    }

    /// Records an actor so the engine can stop and join it.
    pub(crate) fn track<M: Message>(&mut self, actor: &ActorRef<M>, handle: JoinHandle<()>) {
        self.actors.push((actor.get_cell(), handle));
    }

    /// Gives workers a signal that changes when the drain grace period expires.
    pub(crate) fn cancellation(&self) -> watch::Receiver<bool> {
        self.cancel.subscribe()
    }

    /// Detaches the claimed workspace before deleting it.
    ///
    /// A new engine can recreate the original path as soon as the rename
    /// succeeds. The recursive deletion only touches the detached directory.
    pub(crate) async fn remove_workspace(mut self, workspace: &Path) -> Result<()> {
        let parent = workspace
            .parent()
            .ok_or_else(|| eyre::eyre!("workspace must have a parent directory"))?;
        let detached = loop {
            let index = NEXT_CLEANUP.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".cleanup-{}-{index}", std::process::id()));
            match tokio::fs::create_dir(&path).await {
                Ok(()) => break path,
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err).wrap_err("reserve workspace cleanup directory"),
            }
        };
        if let Err(err) = tokio::fs::rename(workspace, detached.join("workspace")).await {
            let _ = tokio::fs::remove_dir(&detached).await;
            return Err(err).wrap_err("detach engine workspace");
        }
        self.marker.take();
        tokio::fs::remove_dir_all(detached)
            .await
            .wrap_err("remove detached engine workspace")
    }

    /// Closes admission, drains or cancels dispatched work, and flushes final state.
    /// Actor and marker cleanup is attempted even when the drain or flush fails.
    async fn shutdown(
        mut self,
        scheduler: ActorRef<SchedulerMsg>,
        state: ActorRef<StateMsg>,
        grace: Duration,
    ) -> Result<()> {
        // Send the request before starting the timer, including for zero grace.
        let (reply, drained) = tokio::sync::oneshot::channel();
        let drain = match scheduler.cast(SchedulerMsg::BeginShutdown(reply.into())) {
            Ok(()) => {
                let mut drained = drained;
                match timeout(grace, &mut drained).await {
                    Ok(result) => result.wrap_err("scheduler stopped before draining"),
                    Err(_) => {
                        self.cancel.send_replace(true);
                        match timeout(FINALIZE_TIMEOUT, drained).await {
                            Ok(result) => result.wrap_err("scheduler stopped before cancellation"),
                            Err(err) => Err(eyre::eyre!("timed out cancelling active jobs: {err}")),
                        }
                    }
                }
            }
            Err(_) => Err(eyre::eyre!("could not close scheduler admission")),
        };
        if drain.is_err() {
            self.cancel.send_replace(true);
        }
        let flush = timeout(FINALIZE_TIMEOUT, async {
            call!(state, StateMsg::Flush)??;
            Ok(())
        })
        .await
        .wrap_err("timed out flushing engine state")
        .and_then(|result| result);
        let stopped = self.teardown().await;
        drain.and(flush).and(stopped)
    }

    /// Stops tracked actors in reverse startup order and removes the marker after joining.
    /// On an unresponsive actor, requests forced termination and leaves the marker
    /// in place if actor exit still cannot be confirmed.
    pub(crate) async fn teardown(&mut self) -> Result<()> {
        if !matches!(
            timeout(FINALIZE_TIMEOUT, self.join_actors()).await,
            Ok(Ok(()))
        ) {
            let children: Vec<_> = self
                .actors
                .iter()
                .flat_map(|(cell, _)| cell.get_children())
                .collect();
            for cell in &children {
                cell.kill();
            }
            self.kill_actors();
            let exited = timeout(FINALIZE_TIMEOUT, async {
                for cell in children {
                    cell.wait(None).await?;
                }
                self.join_actors().await
            })
            .await;
            if !matches!(exited, Ok(Ok(()))) {
                // Preserve the marker as a sign that cleanup could not be confirmed.
                self.marker.take();
                return Err(eyre::eyre!("engine actors did not exit during shutdown"));
            }
        }
        if let Some(marker) = &self.marker {
            match tokio::fs::remove_file(marker).await {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err).wrap_err("remove engine running marker"),
            }
            self.marker.take();
        }
        Ok(())
    }

    /// Sends stop signals and awaits the actor handles, with the supervisor last.
    async fn join_actors(&mut self) -> Result<()> {
        while let Some((cell, handle)) = self.actors.last_mut() {
            cell.stop(None);
            let result = handle.await;
            self.actors.pop();
            result?;
        }
        Ok(())
    }

    /// Requests forced termination for the tracked actors.
    fn kill_actors(&self) {
        self.cancel.send_replace(true);
        for (cell, _) in self.actors.iter().rev() {
            cell.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::StateActor;
    use object_store::local::LocalFileSystem;
    use ractor::{Actor, ActorProcessingErr};
    use std::sync::Arc;

    struct ActiveScheduler;

    #[ractor::async_trait]
    impl Actor for ActiveScheduler {
        type Msg = SchedulerMsg;
        type State = watch::Receiver<bool>;
        type Arguments = watch::Receiver<bool>;

        /// Retains the cancellation receiver so the mock can hold the drain open.
        async fn pre_start(
            &self,
            _myself: ActorRef<Self::Msg>,
            cancel: Self::Arguments,
        ) -> Result<Self::State, ActorProcessingErr> {
            Ok(cancel)
        }

        /// Represents a dispatched job that finishes only after cancellation.
        async fn handle(
            &self,
            _myself: ActorRef<Self::Msg>,
            message: Self::Msg,
            cancel: &mut Self::State,
        ) -> Result<(), ActorProcessingErr> {
            if let SchedulerMsg::BeginShutdown(waiter) = message {
                cancel.wait_for(|cancelled| *cancelled).await?;
                let _ = waiter.send(());
            }
            Ok(())
        }
    }

    /// Forced shutdown signals an active job and joins the actors before removing the marker.
    #[tokio::test]
    async fn expired_grace_signals_cancellation_before_cleanup() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let mut resources = Resources::claim(dir.path())?;
        let signal = resources.cancellation();
        let (scheduler, handle) = Actor::spawn(None, ActiveScheduler, signal.clone()).await?;
        resources.track(&scheduler, handle);
        let (state, handle) = Actor::spawn(
            None,
            StateActor,
            Arc::new(LocalFileSystem::new_with_prefix(dir.path())?),
        )
        .await?;
        resources.track(&state, handle);

        let lifecycle = Lifecycle::new(resources, dir.path().to_owned());
        assert!(!*signal.borrow());
        lifecycle
            .shutdown(scheduler.clone(), state.clone(), Duration::ZERO)
            .await?;
        assert!(*signal.borrow());
        assert_eq!(
            scheduler.get_cell().get_status(),
            ractor::ActorStatus::Stopped
        );
        assert_eq!(state.get_cell().get_status(), ractor::ActorStatus::Stopped);
        assert!(dir.path().join("state").is_file());
        assert!(!dir.path().join(CANARY_FILE).exists());
        Ok(())
    }
}

impl Drop for Resources {
    /// Provides best-effort cleanup if startup or the last engine handle is dropped.
    fn drop(&mut self) {
        self.kill_actors();
        if let Some(marker) = self.marker.take() {
            let _ = std::fs::remove_file(marker);
        }
    }
}
