use std::env::set_current_dir;
use std::path::Path;

use eyre::Result;
use tokio::fs;
use tracing::info;

use crate::runtime::Context;

#[tracing::instrument]
pub async fn clean(ctx: &Context) -> Result<()> {
    info!("Cleaning up airport data files and resources");

    // Get our filename
    //
    let fname = ctx.cfg["file"].clone();
    let fname = Path::new(&fname);

    let basename = fname.file_stem().unwrap().to_str().unwrap();

    let parquet = Path::new(&basename).with_extension("parquet");

    // Check the current file mtime (if it exists)
    //
    let target_dir = Path::new(&ctx.cfg["datalake"]).join("files");

    // Let us move into the final destination
    //
    set_current_dir(&target_dir)?;

    if parquet.exists() {
        info!("Deleting file: {:?}", parquet);
        Ok(fs::remove_file(&parquet).await?)
    } else {
        info!("No file to delete");
        Ok(())
    }
}
