//! Macros creation and deletion.
//!
use crate::runtime::Context;

#[tracing::instrument(skip(ctx))]
async fn add_macro_dist2d(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;

    let r1 = r##"
CREATE FUNCTION dist_2d AS (dx, dy, px, py) ->
  ceil(geoDistance(dx,dy,px,py));
    "##;

    Ok(dbh.execute(r1).await?)
}

#[tracing::instrument(skip(ctx))]
async fn add_macro_dist3d(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;

    let r2 = r##"
CREATE FUNCTION dist_3d AS (dx, dy, dz, px, py, pz) ->
  ceil(sqrt(pow(dist_2d(dx,dy,px,py), 2) + pow((dz-pz), 2)));
    "##;

    Ok(dbh.execute(r2).await?)
}

#[tracing::instrument(skip(ctx))]
async fn add_which_timezone(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;

    let r2 = r##"
 CREATE FUNCTION which_timezone AS id -> (
    SELECT timezone
    FROM deployments
    WHERE install_id = id
)"##;
    Ok(dbh.execute(r2).await?)
}

/// Adds mathematical macros to the database for distance calculations.
///
/// ### Details
///
/// This function creates two user-defined functions in the ClickHouse database:
/// - `dist_2d`: Calculates horizontal geodesic distance between two points
/// - `dist_3d`: Calculates three-dimensional distance between two points
///
/// ### Errors
///
/// Returns an error if the macros cannot be created, for example, due to:
/// - Database connection issues
/// - Insufficient privileges
/// - Invalid SQL syntax
/// - Existing functions with the same names
///
/// ### References
///
/// - ClickHouse documentation for user-defined functions
///
#[tracing::instrument(skip(ctx))]
pub async fn add_macros(ctx: &Context) -> eyre::Result<()> {
    add_macro_dist2d(ctx).await?;
    add_macro_dist3d(ctx).await?;
    add_which_timezone(ctx).await?;
    Ok(())
}

// -----

#[tracing::instrument(skip(ctx))]
async fn remove_macro_dist2d(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;

    let r1 = r##"
DROP FUNCTION IF EXISTS dist_2d;
    "##;

    Ok(dbh.execute(r1).await?)
}

#[tracing::instrument(skip(ctx))]
async fn remove_macro_dist3d(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;

    let r2 = r##"
DROP FUNCTION IF EXISTS dist_3d;
    "##;

    Ok(dbh.execute(r2).await?)
}

#[tracing::instrument(skip(ctx))]
async fn remove_which_timezone(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;

    let r = r##"
DROP FUNCTION IF EXISTS which_timezone;
    "##;

    Ok(dbh.execute(r).await?)
}

/// Removes mathematical macros from the database.
///
/// This function drops the user-defined functions (`dist_2d` and `dist_3d`)
/// from the ClickHouse database. These functions are used for various distance calculations.
///
/// ### Details
///
/// Removes the following functions:
/// - `dist_2d`: Function for calculating horizontal geodesic distance
/// - `dist_3d`: Function for calculating three-dimensional distance
///
/// ### Errors
///
/// Returns an error if the macros cannot be removed, for example, due to:
/// - Database connection issues
/// - Insufficient privileges
/// - Non-existent functions
///
/// ### References
///
/// - ClickHouse documentation for user-defined functions
///
#[tracing::instrument(skip(ctx))]
pub async fn remove_macros(ctx: &Context) -> eyre::Result<()> {
    remove_which_timezone(ctx).await?;
    remove_macro_dist3d(ctx).await?;
    remove_macro_dist2d(ctx).await?;
    Ok(())
}

