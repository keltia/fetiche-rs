//! Main module for fetching files.
//!
use std::env;
use std::path::Path;
use std::time::{Duration, SystemTime};

use eyre::Result;
use reqwest::redirect::Policy;
use tokio::fs;

use crate::error::Status;
use crate::runtime::Context;
use crate::USER_AGENT;

const ONE_DAY: Duration = Duration::from_hours(24);

/// Fetch the main file, then convert it into parquet.
///
#[tracing::instrument]
pub async fn fetch(ctx: &Context) -> Result<usize> {
    let base_url = ctx.cfg["base_url"];
    if base_url.is_empty() {
        return Err(Status::BaseUrlCannotBeEmpty.into());
    }

    let base_url = if !base_url.ends_with('/') {
        base_url.clone() + "/"
    } else {
        base_url
    };

    let fname = ctx.cfg["file"].clone();
    let basename = Path::new(&fname)
        .file_stem()
        .unwrap_or("csv".into())
        .to_str()
        .unwrap();

    let current = Path::new(ctx.cfg["datalake"])
        .join("files")
        .join(&fname)
        .with_extension("parquet");
    let current_st = fs::metadata(&current).await?;
    let mtime = current_st.modified()?;

    // Do we have a file that is older than one day?
    //
    let now = SystemTime::now();
    if now.duration_since(mtime)? > ONE_DAY {
        // We need to fetch a new version of the file.
        //
        let proxy = env::var("http_proxy").ok();
        let output = Path::new(&basename).with_extension("parquet");
        let output_st = fs::metadata(&output).await?;

        let url = format!("{}{}", base_url, fname);
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .gzip(true)
            .redirect(Policy::limited(5))
            .connect_timeout(Duration::from_secs(10))
            .build()?;
        let resp = client.get(&url).send().await?;
        let resp = resp.bytes().await?;

        let tempdir = tempfile::tempdir()?;
        fs::write(&tempdir.path().join(fname), resp).await?;
    }

    Ok(0)
}
