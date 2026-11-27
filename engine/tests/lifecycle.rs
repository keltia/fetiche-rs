//! Public-API tests for engine startup, workspace isolation, and shutdown.

use std::path::PathBuf;

use eyre::Result;
use fetiche_engine::token::AsdToken;
use fetiche_engine::{Engine, EngineMode, JobStatus, TokenMsg, TokenStatus, TokenType};
use tokio::fs;

const FILE_JOB: &str = r#"
    name = "local file"
    producer = { Read = "input.txt" }
    middle = [{ Tee = "copy.txt" }]
    output = { Save = "output.txt" }
"#;

/// Exercises the token actor through engine clones and verifies persistence after restart.
#[tokio::test]
async fn token_actor_shares_storage_and_persists_across_engine_restarts() -> Result<()> {
    let home = tempfile::tempdir()?;
    let config = write_config(home.path()).await?;
    let config = config.to_str().unwrap();
    let engine = Engine::load(config, EngineMode::Single).await?;
    let clone = engine.clone();
    let key = "asd_account";
    let value = "test-token-value";
    let mut token = AsdToken::default();
    token.token = value.into();

    let missing = engine.load_token(key).await.unwrap_err();
    assert!(matches!(
        missing.downcast_ref(),
        Some(TokenStatus::NotFound(_))
    ));
    let unknown = engine
        .store_token("unsupported", TokenType::AsdToken(token.clone()))
        .await
        .unwrap_err();
    assert!(matches!(
        unknown.downcast_ref(),
        Some(TokenStatus::Unknown(_))
    ));
    engine.store_token(key, TokenType::AsdToken(token)).await?;
    let TokenType::AsdToken(loaded) = clone.load_token(key).await?;
    assert_eq!(loaded.token, value);
    assert_eq!(ractor::call!(clone.tokens, TokenMsg::Count)?, 1);
    assert_eq!(ractor::call!(clone.tokens, TokenMsg::List)?.len(), 1);
    assert!(ractor::call!(clone.tokens, TokenMsg::Path)?.ends_with("/tokens"));
    let listing = clone.list_tokens().await?;
    assert!(listing.contains(key));
    assert!(!listing.contains(value));

    engine.shutdown().await?;
    assert!(clone.load_token(key).await.is_err());
    let restarted = Engine::load(config, EngineMode::Single).await?;
    let TokenType::AsdToken(loaded) = restarted.load_token(key).await?;
    assert_eq!(loaded.token, value);
    restarted.cleanup().await?;
    Ok(())
}

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

/// Checks the public configuration paths and the embedded command catalogue.
#[tokio::test]
async fn engine_exposes_configuration_paths_and_command_catalogue() -> Result<()> {
    let home = tempfile::tempdir()?;
    let config = write_config(home.path()).await?;
    let engine = Engine::load(config.to_str().unwrap(), EngineMode::Single).await?;
    for (actual, expected) in [
        (engine.config_file()?, config),
        (engine.sources_file()?, home.path().join("sources.hcl")),
        (engine.state_file()?, home.path().join("state")),
    ] {
        assert_eq!(
            fs::canonicalize(actual).await?,
            fs::canonicalize(expected).await?
        );
    }
    let commands = engine.list_commands()?;
    for expected in ["fetch", "convert", "save"] {
        assert!(commands.contains(expected));
    }
    engine.cleanup().await?;
    Ok(())
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
