use std::env::set_current_dir;
use std::path::{Path, PathBuf};

use eyre::Result;
use tokio::fs;
use tracing::info;

use crate::runtime::Context;

#[tracing::instrument]
pub async fn cmd_clean(ctx: &Context) -> Result<()> {
    info!("Cleaning up airport data files and resources");

    // Check the current file mtime (if it exists)
    //
    let target_dir = Path::new(&ctx.cfg["datalake"]).join("files");
    set_current_dir(&target_dir)?;

    // Get our filenames
    //
    let sources = ctx.cfg["sources"]
        .clone()
        .split(",")
        .map(|s| s.to_owned())
        .collect::<Vec<_>>();

    for fname in sources.into_iter() {
        let basename = Path::new(&fname);
        let _ = clean_one(&basename).await?;
    }
    Ok(())
}

#[tracing::instrument]
async fn clean_one(fname: &Path) -> Result<()> {
    let parquet = fname.with_extension("parquet");
    if parquet.exists() {
        info!("clean_one file={:?}", parquet);
        return Ok(fs::remove_file(&parquet).await?);
    }
    info!("clean_one err=no file");
    Ok(())
}
