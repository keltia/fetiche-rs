//! Module to fetch the Aeroscope data using the HTTP API
//!

use crate::Context;

use anyhow::{bail, Result};
use log::{debug, info, trace};
use serde::Deserialize;

/// Access token derived from username/password
///
#[derive(Debug, Deserialize)]
struct Token {
    /// Token (SHA-256 or -512 data I guess)
    access_token: String,
}

/// Fetch the access token linked to the given login/password
///
fn fetch_token(ctx: &Context) -> Result<String> {
    let cfg = &ctx.cfg;
    let client = &ctx.client;

    trace!("Fetching token…");
    // Prepare our data
    //
    let body = format!(
        "{{\"username\": \"{}\", \"password\": \"{}\"}}",
        cfg.login, cfg.password
    );

    // fetch token
    //
    let url = format!("{}/login", cfg.base_url);
    let resp = client
        .post(url)
        .header("content-type", "application/json")
        .body(body)
        .send();

    let resp = resp?.text()?;

    let res: Token = serde_json::from_str(&resp)?;
    debug!("{:?}", res);
    Ok(res.access_token)
}

/// Using the access token obtained through `fetch_token()`, fetch the given CSV data
///
pub fn fetch_csv(ctx: &Context) -> Result<String> {
    info!("Fetch data from network…");
    // First call to gen auth token
    //
    let token = fetch_token(ctx)?;

    let cfg = &ctx.cfg;
    let client = &ctx.client;

    // Use the token to authenticate ourselves
    //
    let url = format!("{}/drone/get", &cfg.base_url);
    let resp = client
        .get(url)
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", token))
        .send();

    match resp {
        Ok(resp) => Ok(resp.text().unwrap()),
        Err(e) => bail!("HTTP error: {}", e),
    }
}
