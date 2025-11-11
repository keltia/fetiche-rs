//! This hold the code for the `status` command.
//!

use eyre::Result;

use fetiche_client::EngineSingle;

#[tracing::instrument(skip(e))]
pub async fn status_of_workspace(e: &mut EngineSingle) -> Result<()> {
    let e = e.inner();
    let home = e.home.clone();
    let ws = e.ws();
    let dirs = ws.list().await?;

    println!("Status:");
    println!("  Engine: {}", e.version());
    println!("  Home: {}", home);
    println!("  Workdir: {:?}", ws.path());
    println!("  Running jobs:");
    for dir in dirs {
        println!("    {}", dir);
    }
    Ok(())
}
