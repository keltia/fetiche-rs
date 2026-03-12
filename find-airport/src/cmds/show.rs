use std::env::set_current_dir;
use std::path::Path;

use crate::cmds::{read_parquet_size, Work, WorkStatus};
use crate::runtime::Context;
use eyre::Result;
use futures::future::join_all;
use jiff::Timestamp;
use tokio::fs;

#[tracing::instrument(skip(ctx))]
pub async fn cmd_show(ctx: &Context) -> Result<Vec<Work>> {
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

    let files = sources.into_iter().map(|fname| {
        let fname = fname.clone();

        async move {
            let basename = Path::new(&fname);
            show_one(&basename).await.unwrap_or(Work::default())
        }
    }).collect::<Vec<_>>();

    let files = join_all(files).await;
    Ok(files)
}

#[tracing::instrument]
async fn show_one(fname: &Path) -> Result<Work> {
    let parquet = fname.with_extension("parquet");
    if parquet.exists() {
        let st = fs::metadata(&parquet).await?;
        let rows = read_parquet_size(&parquet).await?;
        let mtime = Timestamp::try_from(st.modified()?)?;

        Ok(Work {
            status: WorkStatus::Present,
            name: parquet.to_string_lossy().to_string(),
            mtime,
            size: st.len(),
            rows,
        })
    } else {
        Ok(Work::default())
    }
}
