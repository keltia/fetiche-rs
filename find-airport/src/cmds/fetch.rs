//! Main module for fetching files.
//!

use std::env::set_current_dir;
use std::fmt::Debug;
use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eyre::Result;
use futures::future::join_all;
use polars::prelude::*;
use reqwest::redirect::Policy;
use tokio::io::AsyncRead;
use tokio::{fs, join};
use tracing::{info, trace, warn};

use crate::cmds::{read_parquet_size, Work, WorkStatus};
use crate::error::Status;
use crate::runtime::Context;
use crate::USER_AGENT;

const ONE_DAY: Duration = Duration::from_hours(24);

/// Fetch the main file, then convert it into parquet.
///
#[tracing::instrument]
pub async fn fetch(ctx: &Context) -> Result<Vec<Work>> {
    let base_url = ctx.cfg["base_url"].clone();
    if base_url.is_empty() {
        return Err(Status::BaseUrlCannotBeEmpty.into());
    }

    let base_url = if !base_url.ends_with('/') {
        base_url.clone() + "/"
    } else {
        base_url
    };

    dbg!(&ctx.cfg);
    // Get our filenames
    //
    let srcs = ctx.cfg["sources"].clone();

    // Check the current file mtime (if it exists)
    //
    let target_dir = Path::new(&ctx.cfg["datalake"]).join("files");

    // Let us move into the final destination
    //
    set_current_dir(&target_dir)?;

    // Check and update if necessary each file in the list
    //
    let list: Vec<_> = srcs
        .split(",")
        .map(|fname| {
            let base_url = base_url.clone();

            async move {
                let current = match fetch_one(&base_url, fname).await {
                    Ok(work) => work,
                    Err(e) => {
                        warn!("error={}", e.to_string());
                        return Work::default();
                    }
                };

                // Now look at the file we got
                //
                let current_st = match fs::metadata(&current.name).await {
                    Ok(st) => st,
                    Err(e) => {
                        warn!("error={}", e.to_string());
                        return Work::default();
                    }
                };
                let mtime = current_st.modified().unwrap();
                info!("size={}, mtime={:?}", current_st.len(), mtime);

                let bytes = match read_parquet_size(&current.name).await {
                    Ok(st) => st,
                    Err(e) => {
                        warn!("error={}", e.to_string());
                        return Work::default();
                    }
                };
                info!("file={} rows={:?}", &current.name, bytes);
                current
            }
        })
        .collect();

    let list = join_all(list).await;

    Ok(list)
}

/// Fetch one file into the configured directory, checking mtime, etc.
///
#[tracing::instrument(skip(base_url))]
async fn fetch_one(base_url: &str, fname: &str) -> Result<Work> {
    let mut status: WorkStatus;
    let mut bytes = 0u64;

    // Get our filename
    //
    let basename = Path::new(&fname);

    // Compare the parquet file mtime with now
    //
    let current = basename.with_extension("parquet");

    // Get mtime
    //
    let mtime = if current.exists() {
        let current_st = fs::metadata(&current).await?;
        let mtime = current_st.modified()?;
        bytes = current_st.len();

        info!("File found, size={bytes}, mtime={mtime:?}");
        status = WorkStatus::Present;
        mtime
    } else {
        warn!("No file found, fetching.");
        status = WorkStatus::Refreshed;
        UNIX_EPOCH
    };

    // Do we have a file that is older than one day?
    //
    let now = SystemTime::now();
    if now.duration_since(mtime)? > ONE_DAY {
        info!("Fetching new version.");

        // We need to fetch a new version of the file.
        //
        let output = basename.with_extension("csv");

        let tempdir = tempfile::tempdir()?;
        let output = tempdir.path().join(output);
        info!("Writing to {:?}", output);

        let url = format!("{}{}", base_url, fname);
        let _ = fetch_file(&url, &output).await?;

        let input = output;

        let output = basename.with_extension("parquet");

        info!("Converting to parquet in {:?}", output);
        let _ = convert_into_parquet(&input, &output).await?;
    }

    let current_st = fs::metadata(&current).await?;
    let mtime = current_st.modified()?;
    bytes = current_st.len();

    let rows = read_parquet_size(&current).await?;
    info!("Parquet length: {:?} records", rows);

    Ok(Work {
        status,
        name: fname.to_string(),
        mtime,
        size: bytes,
        rows,
    })
}

#[tracing::instrument]
async fn read_parquet_size<P>(fname: P) -> Result<usize>
where
    P: AsRef<Path> + Debug,
{
    let fh = File::open(fname)?;
    let mut rdr = ParquetReader::new(fh);
    Ok(rdr.num_rows()?)
}

#[tracing::instrument]
async fn fetch_file(url: &str, output: &Path) -> Result<Metadata> {
    let client = reqwest::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .gzip(true)
        .redirect(Policy::limited(5))
        .connect_timeout(Duration::from_secs(10))
        .build()?;
    let resp = client.get(url).send().await?;
    let resp = resp.bytes().await?;

    fs::write(&output, resp).await?;

    Ok(fs::metadata(&output).await?)
}

#[tracing::instrument]
async fn convert_into_parquet<P>(input: P, output: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    let input = input.as_ref();
    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .with_ignore_errors(true)
        .try_into_reader_with_file_path(Some(input.into()))?
        .finish()?;

    let mut out = File::create(&output)?;
    let _ = ParquetWriter::new(&mut out)
        .with_compression(ParquetCompression::Zstd(Some(ZstdLevel::try_new(8)?)))
        .with_statistics(StatisticsOptions::default())
        .finish(&mut df)?;
    Ok(())
}
