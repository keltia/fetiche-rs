//! Anything related to ACUTE site management
//!

use cached::proc_macro::cached;
use chrono::{DateTime, Datelike, Utc};
use klickhouse::{Client, QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tracing::debug;

use crate::cmds::CmdError;

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
#[tracing::instrument(skip(dbh))]
#[cached(key = "String", result = true, convert = r#"{format!("{}", site)}"#)]
pub async fn find_site(dbh: &Client, site: &str) -> eyre::Result<Site> {
    let r = r##"
    SELECT * from sites WHERE name = $1
    "##;
    let q = QueryBuilder::new(r).arg(site);
    let site = match dbh.query_one::<Site>(q).await {
        Ok(site) => site,
        Err(_) => return Err(CmdError::UnknownSite(site.to_string()).into()),
    };
    Ok(site)
}

/// Searches for a specific site in the database based on the provided site ID.
///
/// This function utilizes caching to improve performance, storing the result
/// of previous queries and reusing them for subsequent similar requests.
/// The function logs the operation using `tracing` instrumentation for
/// better debugging and observability.
///
/// ### Arguments
/// - `dbh` - A reference to the ClickHouse database client.
/// - `id` - The unique identifier of the site to search for.
///
/// ### Returns
/// - `Ok(Site)`: If the site is found in the database.
/// - `Err(eyre::Report)`: Returns an error if the site cannot be found or if an issue occurs during the query.
///
/// ### Notes
/// - This function assumes the `sites` table has a column named `id` as the primary key.
/// - The query runs against the ClickHouse database using `QueryBuilder`.
///
/// ### Example
/// ```rust
/// # use your_crate_name::{Client, find_site_by_id};
/// let dbh = Client::new(); // Assume client is properly initialized
/// let site_id = 1;
/// let site = find_site_by_id(&dbh, site_id).await?;
/// println!("Found site: {}", site);
/// ```
///
/// ### Caching
/// - This function leverages the `cached` crate to cache the result of queries,
///   improving performance for frequently accessed site IDs.
///
/// ### Instrumentation
/// - Tracing instrumentation is added to provide detailed logs of the function execution.
///
#[tracing::instrument(skip(dbh))]
#[cached(key = "String", result = true, convert = r#"{format!("{}", id)}"#)]
pub async fn find_site_by_id(dbh: &Client, id: u32) -> eyre::Result<Site> {
    let r = r##"
    SELECT * from sites WHERE id = $1
    "##;
    let q = QueryBuilder::new(r).arg(id);
    let site = match dbh.query_one::<Site>(q).await {
        Ok(site) => site,
        Err(_) => return Err(CmdError::UnknownSiteId(id).into()),
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
#[tracing::instrument(skip(dbh))]
pub async fn enumerate_sites(dbh: &Client, day: DateTime<Utc>) -> eyre::Result<Vec<Site>> {
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

/// Matches an antenna to a site name for a specific date.
///
/// This function queries the database to find which site an antenna was deployed at
/// during the specified date by checking the deployments table.
///
/// ### Arguments
/// - `ctx` - A reference to the application's context containing the database connection.
/// - `day` - The date for which to check the antenna's deployment location.
/// - `antenna` - The name/identifier of the antenna to look up.
///
/// ### Returns
/// - `Ok(String)`: The name of the site where the antenna was deployed on the specified date.
/// - `Err(eyre::Report)`: Returns an error if the antenna deployment cannot be found or if an issue occurs during the query.
///
/// ### Notes
/// - The function checks the deployments table for records matching the antenna name and date range.
/// - Uses caching to improve performance for repeated lookups of the same antenna/date combination.
///
/// ### Caching
/// - Results are cached using the antenna name and date as the cache key.
/// - The cache helps avoid redundant database queries for frequently accessed combinations.
///
#[cached(
    key = "String",
    convert = r#"{ format!("{}{}", day,antenna.to_string()) }"#,
    result = true
)]
#[tracing::instrument(skip(dbh))]
pub async fn match_site(dbh: &Client, day: DateTime<Utc>, antenna: &str) -> eyre::Result<String> {
    #[derive(Deserialize, Row, Serialize)]
    struct Depl {
        pub site_name: String,
    }

    let q = r##"
SELECT site_name
FROM deployments AS d
WHERE d.antenna_name = $1 AND $2 BETWEEN d.start_at AND d.end_at
    "##;

    let qb = QueryBuilder::new(q)
        .arg(antenna)
        .arg(day);

    let depl = dbh.query_one::<Depl>(qb).await?;
    let site = depl.site_name;
    Ok(site)
}
