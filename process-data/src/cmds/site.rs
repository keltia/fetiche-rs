//! Anything related to ACUTE site management
//!

use chrono::{Datelike, DateTime, Utc};
use clickhouse::Row;
use futures::future::join_all;
use serde::{Deserialize, Serialize};

use crate::config::Context;
use crate::error::Status;

/// A site, as reflected in the "sites" table
#[derive(Debug, Deserialize, Row, Serialize)]
pub struct Site {
    /// auto-increment ID
    pub id: u32,
    /// Short name of location (e.g. "BUC")
    pub name: String,
    /// Places code
    pub code: String,
    /// basename of files (e.g. "Bucharest")
    pub basename: String,
    /// Site coordinates latitude
    pub latitude: f32,
    /// Site coordinates longitude
    pub longitude: f32,
    /// Reference altitude
    pub ref_alt: f32,
}

/// Find a given site, id, location,, etc. frm database
///
pub async fn find_site(ctx: &Context, site: &str) -> eyre::Result<Site> {
    let dbh = ctx.db();

    // Load locations from DB
    //
    let r = r##"
    SELECT * from sites WHERE name = ?
    "##;
    let name = site.clone();
    let site = match dbh.query(r).bind(name).fetch_one::<Site>().await {
        Ok(site) => site,
        Err(e) => return Err(Status::UnknownSite(name.to_string()).into()),
    };
    Ok(site)
}

/// Now, for a given day, find all sites that have data
///
pub async fn enumerate_sites(ctx: &Context, day: DateTime<Utc>) -> eyre::Result<Vec<Site>> {
    let dbh = ctx.db();

    let day_tag = format!("{:4}{:02}{:02}", day.year(), day.month(), day.day());
    let r = r##"
SELECT sites.name AS site_id
FROM
    sites, installations
WHERE (sites.id = installations.site_id) AND
    (toDateTime(?) BETWEEN installations.start_at AND
    installations.end_at)
    "##;

    // Fetch all site IDs for this specific day
    //
    let sites = dbh.query(r).bind(day_tag).fetch_all::<String>().await?;

    // Now create our array of sites for all ids
    //
    let sites = sites
        .iter()
        .map(|name| async { find_site(ctx, name).await.unwrap() })
        .collect::<Vec<_>>();
    let sites = join_all(sites).await;

    Ok(sites)
}
