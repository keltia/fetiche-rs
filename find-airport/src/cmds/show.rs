use std::env::set_current_dir;
use std::path::Path;

use crate::cmds::read_parquet_size;
use crate::runtime::Context;
use eyre::Result;
use jiff::Timestamp;
use tokio::fs;

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
        let rows = read_parquet_size(&parquet).await?;
        let mtime = Timestamp::try_from(st.modified()?)?;
        println!("file={parquet:?} size={} rows={} mtime=\"{}\"", st.len(), rows, mtime.strftime("%Y-%m-%d %H:%M:%S"));
    }
    Ok(())
}
