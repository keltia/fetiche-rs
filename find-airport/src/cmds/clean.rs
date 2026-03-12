use std::env::set_current_dir;
use std::path::Path;

use crate::cmds::{read_parquet_size, Work, WorkStatus};
use crate::runtime::Context;
use eyre::Result;
use futures::future::join_all;
use jiff::Timestamp;
use tokio::fs;
use tracing::info;

#[tracing::instrument]
pub async fn cmd_clean(ctx: &Context) -> Result<Vec<Work>> {
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

    let files = sources.into_iter().map(|fname| {
        let fname = fname.clone();

        async move {
            let basename = Path::new(&fname);
            clean_one(&basename).await.unwrap_or(Work::default())
        }
    }).collect::<Vec<_>>();

    let files = join_all(files).await;

    Ok(files)
}

#[tracing::instrument]
async fn clean_one(fname: &Path) -> Result<Work> {
    let parquet = fname.with_extension("parquet");
    if parquet.exists() {
        info!("clean_one file={:?}", parquet);
        let st = fs::metadata(&parquet).await?;
        let rows = read_parquet_size(&parquet).await?;
        let mtime = Timestamp::try_from(st.modified()?)?;

        let status = match fs::remove_file(&parquet).await {
            Ok(_) => WorkStatus::Removed,
            Err(e) => {
                info!("clean_one err={}", e);
                WorkStatus::Unknown
            }
        };
        let work = Work {
            status,
            name: parquet.to_string_lossy().to_string(),
            size: 0,
            mtime,
            rows: 0,
        };
        return Ok(work);
    }
    info!("clean_one err=no file");
    Ok(Work::default())
}
