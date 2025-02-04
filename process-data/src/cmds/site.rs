//! Anything related to ACUTE site management
//!

use cached::proc_macro::cached;
use chrono::{DateTime, Datelike, Utc};
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tracing::debug;

use crate::error::Status;
use crate::runtime::Context;

/// Represents a site entity stored in the database within the `sites` table.
///
/// This structure contains various attributes related to the site, such as its ID,
/// name, coordinates, and other metadata.
///
/// ### Fields
/// - `id`: The auto-incrementing identifier of the site.
/// - `name`: A short name identifying the site (e.g., "BUC").
/// - `code`: The site's associated code (for classification purposes).
/// - `basename`: The base name or descriptive name of the site (e.g., "Bucharest").
/// - `latitude`: The latitude coordinate of the site's location.
/// - `longitude`: The longitude coordinate of the site's location.
/// - `ref_alt`: The reference altitude for the site's location.
///
#[derive(Clone, Debug, Deserialize, Row, Serialize)]
pub struct Site {
    /// auto-increment ID
    pub id: i32,
    /// Short name of location (e.g. "BUC")
    pub name: String,
    /// Places code
    pub code: String,
    /// basename of files (e.g. "Bucharest")
    pub basename: String,
    /// Site coordinates latitude
    pub latitude: f64,
    /// Site coordinates longitude
    pub longitude: f64,
    /// Reference altitude
    pub ref_alt: i32,
    /// timezone name
    pub timezone: String,
}

impl Display for Site {
    /// Implement `Display`.  Just return the name for now.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Searches for a specific site in the database based on the provided site name.
///
/// This function utilizes caching to improve performance, storing the result
/// of previous queries and reusing them for subsequent similar requests.
/// The function logs the operation using `tracing` instrumentation for
/// better debugging and observability.
///
/// ### Arguments
/// - `ctx` - A reference to the application's context containing the database connection.
/// - `site` - The name of the site to search for.
///
/// ### Returns
/// - `Ok(Site)`: If the site is found in the database.
/// - `Err(eyre::Report)`: Returns an error if the site cannot be found or if an issue occurs during the query.
///
/// ### Notes
/// - This function assumes the `sites` table has a column named `name`, and it is unique for identifying sites.
/// - The query runs against the ClickHouse database using `QueryBuilder`.
///
/// ### Example
/// ```rust
/// # use chrono::Utc;
/// # use your_crate_name::{Context, find_site};
/// let ctx = Context::new(); // Assume context is properly initialized
/// let site_name = "BUC";
/// let site = find_site(&ctx, site_name).await?;
/// println!("Found site: {}", site);
/// ```
///
/// ### Caching
/// - This function leverages the `cached` crate to cache the result of queries,
///   improving performance for frequently accessed sites.
///
/// ### Instrumentation
/// - Tracing instrumentation is added to provide detailed logs of the function execution.
///
#[tracing::instrument(skip(ctx))]
#[cached(key = "String", result = true, convert = r#"{format!("{}", site)}"#)]
pub async fn find_site(ctx: &Context, site: &str) -> eyre::Result<Site> {
    let dbh = ctx.db().await;

    // Load locations from DB
    //
    let r = r##"
    SELECT * from sites WHERE name = $1
    "##;
    let q = QueryBuilder::new(r).arg(site);
    let site = match dbh.query_one::<Site>(q).await {
        Ok(site) => site,
        Err(_) => return Err(Status::UnknownSite(site.to_string()).into()),
    };
    Ok(site)
}

/// Retrieves all sites that have data for a specified day.
///
/// This function queries the database to return a list of `Site` entities
/// that have associated data within the `sites` and `installations` tables
/// for the provided date.
///
/// ### Arguments
/// - `ctx` - The application context which holds the database connection.
/// - `day` - A `DateTime<Utc>` object representing the specific day to search for.
///
/// ### Returns
/// - `Ok(Vec<Site>)`: A vector of `Site` entities found for the specified `day`.
/// - `Err(eyre::Report)`: Returns an error if the query fails or there is an issue fetching the data.
///
/// ### Example
///
/// ```rust
/// use chrono::{Utc};
///
/// let day = Utc::now(); // Specify a date
/// let sites = enumerate_sites(&context, day).await?;
/// for site in sites {
///     println!("Site: {}", site);
/// }
/// ```
///
/// ### Notes
/// - The function ensures that the specified day is within the valid
///   start and end dates of site installations.
///
/// ### Instrumentation
/// - This function is instrumented with tracing for debugging purposes.
///
#[tracing::instrument(skip(ctx))]
pub async fn enumerate_sites(ctx: &Context, day: DateTime<Utc>) -> eyre::Result<Vec<Site>> {
    let dbh = ctx.db().await;

    let day_tag = format!("{:4}-{:02}-{:02}", day.year(), day.month(), day.day());
    let r = r##"
SELECT
    DISTINCT(s.id),
    s.name,
    s.code,
    s.basename,
    s.latitude,
    s.longitude,
    s.ref_altitude,
    s.timezone,
FROM
    sites AS s, installations
WHERE (s.id = installations.site_id) AND
    (toDateTime($1) BETWEEN installations.start_at AND
    installations.end_at)
    "##;
    let q = QueryBuilder::new(r).arg(day_tag);

    // Fetch all site IDs for this specific day
    //
    let sites = dbh.query_collect::<Site>(q).await?;
    debug!("sites={:?}", sites);
    Ok(sites)
}
