//! Anything related to ACUTE site management
//!

use std::fmt::{Display, Formatter};

use chrono::{DateTime, Datelike, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config::Context;
use crate::error::Status;

/// A site, as reflected in the "sites" table
#[derive(Clone, Debug, Deserialize, Row, Serialize)]
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

impl Display for Site {
    /// Implement `Display`.  Just return the name for now.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Find a given site, id, location,, etc. frm database
///
#[tracing::instrument(skip(ctx))]
pub async fn find_site(ctx: &Context, site: &str) -> eyre::Result<Site> {
    let dbh = ctx.db();

    // Load locations from DB
    //
    let r = r##"
    SELECT * from sites WHERE name = ?
    "##;
    let site = match dbh.query(r).bind(site).fetch_one::<Site>().await {
        Ok(site) => site,
        Err(_) => return Err(Status::UnknownSite(site.to_string()).into()),
    };
    Ok(site)
}

/// Now, for a given day, find all sites that have data
///
#[tracing::instrument(skip(ctx))]
pub async fn enumerate_sites(ctx: &Context, day: DateTime<Utc>) -> eyre::Result<Vec<Site>> {
    let dbh = ctx.db();

    let day_tag = format!("{:4}-{:02}-{:02}", day.year(), day.month(), day.day());
    let r = r##"
SELECT
    s.id,
    s.name,
    s.code,
    s.basename,
    s.latitude,
    s.longitude,
    s.ref_altitude,
FROM
    sites AS s, installations
WHERE (s.id = installations.site_id) AND
    (toDateTime(?) BETWEEN installations.start_at AND
    installations.end_at)
    "##;

    // Fetch all site IDs for this specific day
    //
    let sites = dbh.query(r).bind(day_tag).fetch_all::<Site>().await?;
    debug!("sites={:?}", sites);
    Ok(sites)
}
