//! Anything related to ACUTE site management
//!
use cached::cached;
use chrono::{DateTime, Datelike, Utc};
use klickhouse::{QueryBuilder, Row};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use tracing::debug;

use crate::cmds::{CmdError, DBVars};
use crate::make_query;
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
/// ```rust,no_run
/// # use process_data::{Context, find_site};
/// async fn example(ctx: &Context) -> eyre::Result<()> {
///     let site = find_site(ctx, "BUC").await?;
///     println!("Found site: {}", site);
///     Ok(())
/// }
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
#[cached(key = "String", convert = r#"{format!("{}", site)}"#)]
pub async fn find_site(ctx: &Context, site: &str) -> eyre::Result<Site> {
    let site = match query_site_by_name(ctx, site).await {
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
/// ```rust,no_run
/// # use process_data::{Context, find_site_by_id};
/// async fn example(ctx: &Context) -> eyre::Result<()> {
///     let site = find_site_by_id(ctx, 1).await?;
///     println!("Found site: {}", site);
///     Ok(())
/// }
/// ```
///
/// ### Caching
/// - This function leverages the `cached` crate to cache the result of queries,
///   improving performance for frequently accessed site IDs.
///
/// ### Instrumentation
/// - Tracing instrumentation is added to provide detailed logs of the function execution.
///
#[tracing::instrument(skip(ctx))]
#[cached(key = "String", convert = r#"{format!("{}", id)}"#)]
pub async fn find_site_by_id(ctx: &Context, id: u32) -> eyre::Result<Site> {
    let site = match query_site_by_id(ctx, id).await {
        Ok(site) => site,
        Err(_) => return Err(CmdError::UnknownSiteId(id).into()),
    };
    Ok(site)
}

/// Queries a site by its name from the database or a test-specific lookup.
///
/// # Arguments
///
/// * `ctx` - A reference to the `Context` instance, which provides access to the database
///           and other contextual resources.
/// * `site` - A string slice representing the name of the site to query.
///
/// # Returns
///
/// Returns a `Result` containing the `Site` object if found, or an `eyre::Error` if an error occurs.
///
/// # Behavior
///
/// * In a test environment (when the `test` configuration is enabled):
///   - If the `site_lookup` is defined in `ctx`, it uses the provided lookup to retrieve the site
///     asynchronously by calling `find_site_by_name`.
///
/// * Outside of a test environment:
///   - Establishes a database connection by calling `ctx.db()`.
///   - Constructs a database query using the site name, with the query defined as
///     `SELECT * FROM sites WHERE name = $1`.
///   - Executes the query and retrieves a single site record.
///
/// # Errors
///
/// * Returns an error if:
///   - There's an issue establishing a database connection.
///   - The query fails to execute.
///   - The site is not found in the database.
///
/// # Example
///
/// ```rust
/// use process_data::{Context, find_site};
///
/// async fn fetch_site(ctx: &Context) -> eyre::Result<()> {
///     let site = find_site(ctx, "example_site").await?;
///     println!("Found site: {site:?}");
///     Ok(())
/// }
/// ```
///
/// # Note
///
/// This function leverages the `make_query!` macro and `QueryBuilder` to construct and execute
/// SQL queries dynamically. Be sure that all necessary context (e.g., `DBVars`, database connection)
/// is properly initialized in `ctx`.
///
#[tracing::instrument(skip(ctx))]
async fn query_site_by_name(ctx: &Context, site: &str) -> eyre::Result<Site> {
    if let Some(lookup) = &ctx.site_lookup {
        return lookup.find_site_by_name(site.to_owned()).await;
    }

    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);
    let r = make_query!(r##"SELECT * from sites WHERE name = $1"##, dbvars);
    let q = QueryBuilder::new(&r).arg(site);
    Ok(dbh.query_one::<Site>(q).await?)
}

/// Queries the site information by its unique identifier from the database.
///
/// This function fetches the site entry associated with the given `id` from the database.
/// Internally, it uses a SQL query to select all columns from the `sites` table where
/// the `id` matches the provided identifier. The result is parsed and returned as a `Site` object.
///
/// This function leverages the `tracing` crate for instrumentation, allowing better observability,
/// and skips the `ctx` parameter during trace recording to avoid capturing sensitive or large data.
///
/// # Arguments
///
/// * `ctx` - The shared `Context` reference, which provides access to application-level state
///   such as database connections and configurations.
/// * `id` - The unique identifier of the site to be queried.
///
/// # Returns
///
/// * `eyre::Result<Site>` - Returns a `Result` wrapping a `Site` object if the query
///   succeeds, or an error of type `eyre::Report` if the operation fails at any step.
///
/// # Behavior
///
/// * During testing (`#[cfg(test)]`), the function looks for an optional `site_lookup`
///   implementation in `ctx` to provide a test-specific way to find the site by `id`. If
///   this exists, it will fetch the site from `site_lookup` instead of the database.
///
/// * In a non-test environment, the function fetches a database handle (`dbh`) using `ctx.db().await`,
///   and prepares and executes a SQL query with the provided `id`.
///
/// * The query is constructed with runtime variables (`DBVars`) derived from the `Context`.
///
/// # Examples
///
/// ```rust
/// use eyre::Result;
/// use process_data::{Context, find_site_by_id};
///
/// async fn example(ctx: &Context) -> Result<()> {
///     let site_id = 42;
///     let site = find_site_by_id(ctx, site_id).await?;
///     println!("Site: {:?}", site);
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns an error in the following cases:
/// * The database connection cannot be established.
/// * The SQL query fails to execute.
/// * The `id` does not correspond to any entry in the `sites` table.
///
/// # Instrumentation
///
/// This function is instrumented with a tracing span (`tracing::instrument`) for better observability
/// and debugging. The `ctx` parameter is excluded from the trace logs to protect potentially sensitive
/// information.
///
/// # Notes
///
/// * Ensure proper database connection management for `ctx` to avoid runtime failures.
/// * The query assumes the existence of a `sites` table with an `id` column in the database schema.
/// * For testing purposes, set `ctx.site_lookup` to perform the lookup without hitting the database.
///
#[tracing::instrument(skip(ctx))]
async fn query_site_by_id(ctx: &Context, id: u32) -> eyre::Result<Site> {
    if let Some(lookup) = &ctx.site_lookup {
        return lookup.find_site_by_id(id).await;
    }

    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);
    let r = make_query!(r##"SELECT * from sites WHERE id = $1"##, dbvars);
    let q = QueryBuilder::new(&r).arg(id);
    Ok(dbh.query_one::<Site>(q).await?)
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
/// use chrono::Utc;
/// use process_data::{Context, enumerate_sites};
///
/// async fn example(ctx: &Context) -> eyre::Result<()> {
///     let sites = enumerate_sites(ctx, Utc::now()).await?;
///     for site in sites {
///         println!("Site: {}", site);
///     }
///     Ok(())
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
    if let Some(lookup) = &ctx.site_lookup {
        return lookup.enumerate_sites(day).await;
    }

    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let day_tag = format!("{:4}-{:02}-{:02}", day.year(), day.month(), day.day());
    let r = make_query!(
        r##"
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
    {workdb}.sites AS s, {workdb}.installations
WHERE
    (s.id = installations.site_id) AND
    (toDateTime($1) BETWEEN installations.start_at AND installations.end_at)
"##,
        dbvars
    );
    let q = QueryBuilder::new(&r).arg(day_tag);

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
async fn query_match_site(
    ctx: &Context,
    day: DateTime<Utc>,
    antenna: &str,
) -> eyre::Result<String> {
    if let Some(lookup) = &ctx.site_lookup {
        return lookup.match_site(day, antenna.to_owned()).await;
    }

    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    #[derive(Deserialize, Row, Serialize)]
    struct Depl {
        pub site_name: String,
    }

    let q = make_query!(
        r##"
SELECT site_name
FROM deployments AS d
WHERE d.antenna_name = $1 AND $2 BETWEEN d.start_at AND d.end_at
    "##,
        dbvars
    );

    let qb = QueryBuilder::new(&q).arg(antenna).arg(day);

    let depl = dbh.query_one::<Depl>(qb).await?;
    let site = depl.site_name;
    Ok(site)
}

#[cached(
    key = "String",
    convert = r#"{ format!("{}{}", day,antenna.to_string()) }"#
)]
#[tracing::instrument(skip(ctx))]
pub async fn match_site(ctx: &Context, day: DateTime<Utc>, antenna: &str) -> eyre::Result<String> {
    query_match_site(ctx, day, antenna).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Duration;

    use chrono::{TimeZone, Utc};

    use super::{CmdError, Site, find_site, find_site_by_id, match_site};
    use crate::runtime::{MockSiteLookup, init_test_context};

    fn test_site(name: &str) -> Site {
        Site {
            id: 1,
            name: name.to_owned(),
            code: "test".to_owned(),
            basename: name.to_owned(),
            latitude: 0.0,
            longitude: 0.0,
            ref_alt: 0,
            timezone: "UTC".to_owned(),
        }
    }

    async fn test_context(lookup: MockSiteLookup) -> crate::runtime::Context {
        let mut ctx = init_test_context(HashMap::new()).await.unwrap();
        ctx.site_lookup = Some(Arc::new(lookup));
        ctx
    }

    #[tokio::test]
    async fn find_site_caches_successes_by_name_and_retries_errors() {
        let missing = "cached-find-site-missing-test".to_owned();
        let calls = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
        let mut lookup = MockSiteLookup::new();
        let call_counts = Arc::clone(&calls);
        let failure_key = missing.clone();
        lookup
            .expect_find_site_by_name()
            .times(4)
            .returning(move |name| {
                let call_counts = Arc::clone(&call_counts);
                let failure_key = failure_key.clone();
                Box::pin(async move {
                    let mut calls = call_counts.lock().unwrap();
                    let count = calls.entry(name.clone()).or_default();
                    *count += 1;
                    if name == failure_key && *count == 1 {
                        return Err(eyre::eyre!("mock database failure"));
                    }
                    Ok(test_site(&name))
                })
            });
        let ctx = test_context(lookup).await;

        let first = find_site(&ctx, "cached-find-site-one-test").await.unwrap();
        let repeated = find_site(&ctx, "cached-find-site-one-test").await.unwrap();
        let second = find_site(&ctx, "cached-find-site-two-test").await.unwrap();
        assert_eq!(first.name, repeated.name);
        assert_eq!(second.name, "cached-find-site-two-test");

        let error = find_site(&ctx, &missing).await.unwrap_err();
        assert!(matches!(
            error.downcast_ref::<CmdError>(),
            Some(CmdError::UnknownSite(name)) if name == &missing
        ));
        assert_eq!(find_site(&ctx, &missing).await.unwrap().name, missing);
        assert_eq!(calls.lock().unwrap().get(&missing), Some(&2));
    }

    #[tokio::test]
    async fn find_site_by_id_caches_successes_and_retries_errors() {
        let missing = 987_651;
        let calls = Arc::new(Mutex::new(HashMap::<u32, usize>::new()));
        let mut lookup = MockSiteLookup::new();
        let call_counts = Arc::clone(&calls);
        lookup
            .expect_find_site_by_id()
            .times(4)
            .returning(move |id| {
                let call_counts = Arc::clone(&call_counts);
                Box::pin(async move {
                    let mut calls = call_counts.lock().unwrap();
                    let count = calls.entry(id).or_default();
                    *count += 1;
                    if id == missing && *count == 1 {
                        return Err(eyre::eyre!("mock database failure"));
                    }
                    Ok(test_site(&format!("site-{id}")))
                })
            });
        let ctx = test_context(lookup).await;

        let first = find_site_by_id(&ctx, 987_649).await.unwrap();
        let repeated = find_site_by_id(&ctx, 987_649).await.unwrap();
        let second = find_site_by_id(&ctx, 987_650).await.unwrap();
        assert_eq!(first.name, repeated.name);
        assert_eq!(second.name, "site-987650");

        let error = find_site_by_id(&ctx, missing).await.unwrap_err();
        assert!(matches!(
            error.downcast_ref::<CmdError>(),
            Some(CmdError::UnknownSiteId(id)) if *id == missing
        ));
        assert_eq!(
            find_site_by_id(&ctx, missing).await.unwrap().name,
            format!("site-{missing}")
        );
        assert_eq!(calls.lock().unwrap().get(&missing), Some(&2));
    }

    #[tokio::test]
    async fn match_site_caches_date_and_antenna_keys_and_retries_errors() {
        let day_one = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let day_two = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        let day_missing = Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap();
        let missing_antenna = "cached-match-site-missing-test".to_owned();
        let calls = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
        let mut lookup = MockSiteLookup::new();
        let call_counts = Arc::clone(&calls);
        let failure_key = format!("{day_missing}{missing_antenna}");
        lookup
            .expect_match_site()
            .times(5)
            .returning(move |day, antenna| {
                let call_counts = Arc::clone(&call_counts);
                let failure_key = failure_key.clone();
                Box::pin(async move {
                    let key = format!("{day}{antenna}");
                    let mut calls = call_counts.lock().unwrap();
                    let count = calls.entry(key.clone()).or_default();
                    *count += 1;
                    if key == failure_key && *count == 1 {
                        return Err(eyre::eyre!("mock database failure"));
                    }
                    Ok(format!("site-{day}-{antenna}"))
                })
            });
        let ctx = test_context(lookup).await;

        let first = match_site(&ctx, day_one, "cached-match-site-antenna-one-test")
            .await
            .unwrap();
        let repeated = match_site(&ctx, day_one, "cached-match-site-antenna-one-test")
            .await
            .unwrap();
        let second_antenna = match_site(&ctx, day_one, "cached-match-site-antenna-two-test")
            .await
            .unwrap();
        let second_day = match_site(&ctx, day_two, "cached-match-site-antenna-one-test")
            .await
            .unwrap();
        assert_eq!(first, repeated);
        assert_ne!(first, second_antenna);
        assert_ne!(first, second_day);

        assert!(
            match_site(&ctx, day_missing, &missing_antenna)
                .await
                .is_err()
        );
        assert!(
            match_site(&ctx, day_missing, &missing_antenna)
                .await
                .is_ok()
        );
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .get(&format!("{day_missing}{missing_antenna}")),
            Some(&2)
        );
    }

    #[tokio::test]
    async fn find_site_allows_concurrent_same_key_misses() {
        let name = "cached-find-site-concurrent-test";
        let calls = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(tokio::sync::Barrier::new(2));
        let mut lookup = MockSiteLookup::new();
        let call_count = Arc::clone(&calls);
        let started_calls = Arc::clone(&started);
        lookup
            .expect_find_site_by_name()
            .times(2)
            .returning(move |_| {
                let call_count = Arc::clone(&call_count);
                let started_calls = Arc::clone(&started_calls);
                Box::pin(async move {
                    call_count.fetch_add(1, Ordering::SeqCst);
                    started_calls.wait().await;
                    Ok(test_site("concurrent-site"))
                })
            });
        let ctx = test_context(lookup).await;
        let results = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(find_site(&ctx, name), find_site(&ctx, name))
        })
        .await
        .expect("concurrent cached lookups timed out");
        let first = results.0.unwrap();
        let second = results.1.unwrap();

        assert_eq!(first.name, second.name);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
