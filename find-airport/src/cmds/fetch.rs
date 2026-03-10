//! Main module for fetching files.
//!

use std::env::set_current_dir;
use std::fs::{File, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eyre::Result;
use jiff::civil::DateTime;
use polars::prelude::*;
use reqwest::redirect::Policy;
use tokio::fs;
use tokio::io::AsyncRead;
use tracing::{info, trace};

use crate::error::Status;
use crate::runtime::Context;
use crate::USER_AGENT;

const ONE_DAY: Duration = Duration::from_hours(24);

/// Fetch the main file, then convert it into parquet.
///
#[tracing::instrument]
pub async fn fetch(ctx: &Context) -> Result<usize> {
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
    let fname = ctx.cfg["file"].clone();
    let basename = Path::new(&fname)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();

    // Check current file mtime (if it exists)
    //
    let target_dir = Path::new(&ctx.cfg["datalake"]).join("files");
    // Let us move into the final destination
    //
    set_current_dir(&target_dir)?;

    let current = target_dir.join(&fname)
        .with_extension("parquet");
    info!("Looking for file: {:?}", current);

    // Get mtime
    //
    let mtime = if current.exists() {
        let current_st = fs::metadata(&current).await?;
        let mtime = current_st.modified()?;
        let bytes = current_st.len();
        info!("File found, size={bytes}, mtime={mtime:?}");
        mtime
    } else {
        info!("No file found, fetching.");
        UNIX_EPOCH
    };

    // Do we have a file that is older than one day?
    //
    let now = SystemTime::now();
    let current = if now.duration_since(mtime)? > ONE_DAY {
        info!("Fetching new version.");

        // We need to fetch a new version of the file.
        //
        let output = Path::new(&basename).with_extension("parquet");

        let tempdir = tempfile::tempdir()?;
        let output = tempdir.path().join(output);
        info!("Writing to {:?}", output);

        let url = format!("{}{}", base_url, fname);
        let st = fetch_file(&url, &output).await?;

        let input = output;

        let output = Path::new(&input).file_stem().unwrap().to_str().unwrap();
        let output = Path::new(output).with_extension("parquet");

        info!("Converting to parquet in {:?}", output);
        let _ = convert_into_parquet(&input, &output).await?;
        output
    } else {
        current
    };

    // Now look at the file we got
    //
    let current_st = fs::metadata(&current).await?;
    let mtime = current_st.modified()?;
    info!("File size: {} bytes, from {:?}", current_st.len(), mtime);

    let bytes = read_parquet_size(&current).await?;
    info!("Parquet length: {:?} records", bytes);

    Ok(bytes)
}

#[tracing::instrument]
async fn read_parquet_size(fname: &PathBuf) -> Result<usize> {
    let fh = File::open(fname)?;
    let mut rdr = ParquetReader::new(fh);
    Ok(rdr.num_rows()?)
}

#[tracing::instrument]
async fn fetch_file(url: &str, output: &Path) -> Result<Metadata> {
    let client =
        reqwest::ClientBuilder::new()
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
async fn convert_into_parquet(input: &Path, output: &Path) -> Result<()> {
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
