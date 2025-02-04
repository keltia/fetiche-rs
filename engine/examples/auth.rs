use fetiche_common::init_logging;
use fetiche_engine::Engine;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_logging("auth", false, false, None)?;

    let engine = Engine::new().await;
    dbg!(&engine);

    let str = engine.list_tokens()?;
    eprintln!("{}", str);
    Ok(())
}
