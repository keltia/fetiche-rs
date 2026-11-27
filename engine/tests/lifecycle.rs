//! Public-API tests for engine startup, workspace isolation, and shutdown.

use std::path::PathBuf;

use eyre::Result;
use fetiche_engine::{Engine, EngineMode, JobStatus};
use tokio::fs;

const FILE_JOB: &str = r#"
    name = "local file"
    producer = { Read = "input.txt" }
    middle = [{ Tee = "copy.txt" }]
    output = { Save = "output.txt" }
"#;

/// Writes an isolated configuration and returns the explicit engine config path.
async fn write_config(home: &std::path::Path) -> Result<PathBuf> {
    let basedir = serde_json::to_string(&home.join("data").to_string_lossy().replace('\\', "/"))?;
    let config = home.join("engine.hcl");
    fs::write(
        &config,
        format!("version = 3\nbasedir = {basedir}\nstorage = {{}}\n"),
    )
    .await?;
    fs::write(home.join("sources.hcl"), "version = 4\nsite = {}\n").await?;
    fs::create_dir(home.join("tokens")).await?;
    Ok(config)
}

/// Runs concurrent jobs with identical relative filenames in separate engine workspaces.
#[tokio::test]
async fn concurrent_engines_isolate_jobs_and_share_shutdown_across_clones() -> Result<()> {
    let first_home = tempfile::tempdir()?;
    let second_home = tempfile::tempdir()?;
    let first_config = write_config(first_home.path()).await?;
    let second_config = write_config(second_home.path()).await?;
    let (mut first, mut second) = tokio::try_join!(
        Engine::load(first_config.to_str().unwrap(), EngineMode::Single),
        Engine::load(second_config.to_str().unwrap(), EngineMode::Single),
    )?;

    drop(first.clone());
    let cwd = std::env::current_dir()?;
    fs::write(first.workdir.join("input.txt"), "first workspace").await?;
    fs::write(second.workdir.join("input.txt"), "second workspace").await?;
    let first_job = first.parse_job(FILE_JOB).await?;
    let second_job = second.parse_job(FILE_JOB).await?;

    let (_, ()) = tokio::try_join!(second.submit_job_and_wait(second_job), async {
        let ticket = first.submit_job(first_job).await?;
        assert_eq!(ticket.rx.await?.status, JobStatus::Succeeded);
        Ok::<_, eyre::Report>(())
    })?;
    assert_eq!(std::env::current_dir()?, cwd);
    for (engine, expected) in [(&first, "first workspace"), (&second, "second workspace")] {
        assert_eq!(
            fs::read_to_string(engine.workdir.join("output.txt")).await?,
            expected
        );
        assert_eq!(
            fs::read_to_string(engine.workdir.join("copy.txt")).await?,
            expected
        );
    }

    // A shutdown must acknowledge a new snapshot even if the previous file was removed.
    fs::remove_file(first_home.path().join("state")).await?;
    let clone = first.clone();
    tokio::try_join!(first.shutdown(), clone.shutdown())?;
    first.shutdown().await?;
    assert!(first_home.path().join("state").is_file());
    assert!(!first.workdir.join("running").exists());
    assert!(second.workdir.join("running").is_file());
    assert!(second.sources().await?.is_empty());
    second.cleanup().await?;
    second.cleanup().await?;
    assert!(!second.workdir.exists());
    Ok(())
}

/// Startup failure releases the marker and permits a subsequent start.
#[tokio::test]
async fn startup_failure_releases_ownership_without_touching_a_live_engine() -> Result<()> {
    let home = tempfile::tempdir()?;
    let config = write_config(home.path()).await?;
    let config = config.to_str().unwrap();
    let state = home.path().join("state");
    fs::write(&state, "invalid snapshot").await?;
    assert!(Engine::load(config, EngineMode::Single).await.is_err());
    let workspace = home
        .path()
        .join("data/var/run")
        .join(std::process::id().to_string());
    assert!(!workspace.join("running").exists());
    assert!(!home.path().join("acutectl.pid").exists());

    fs::remove_file(state).await?;
    let engine = Engine::load(config, EngineMode::Single).await?;
    assert!(Engine::load(config, EngineMode::Single).await.is_err());
    assert!(workspace.join("running").is_file());
    assert!(engine.sources().await?.is_empty());
    engine.shutdown().await?;
    assert!(!workspace.join("running").exists());
    Ok(())
}
