use fetiche_common::init_logging;
use fetiche_engine::Engine;
use tokio::task;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_logging("auth", false, false, None)?;

    let engine = Engine::new().await;
    dbg!(&engine);

    let str = task::spawn_blocking(move || engine.list_tokens().unwrap()).await?;
    eprintln!("{}", str);
    Ok(())
}
