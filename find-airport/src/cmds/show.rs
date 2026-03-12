use std::env::set_current_dir;
use std::path::Path;

use eyre::Result;
use jiff::Timestamp;
use tokio::fs;

use crate::runtime::Context;

#[tracing::instrument(skip(ctx))]
pub async fn cmd_show(ctx: &Context) -> Result<()> {
    // Move ourselves in the right directory.
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
        let _ = show_one(&basename).await?;
    }

    Ok(())
}

#[tracing::instrument]
async fn show_one(fname: &Path) -> Result<()> {
    let parquet = fname.with_extension("parquet");
    if parquet.exists() {
        let st = fs::metadata(&parquet).await?;
        let mtime = Timestamp::try_from(st.modified()?)?;
        println!("file={parquet:?} size={} mtime=\"{}\"", st.len(), mtime.strftime("%Y-%m-%d %H:%M:%S"));
    }
    Ok(())
}
