//! for all drone points:
//!     convert drone location into Point_2D
//!     convert home location into Point_2D
//!     compute distance and store in table
//!     compute 3d distance
//!     insert both in base
//!
//! We also need the following macros defined:
//! `dist_2d`, `dist_3d`
//!
//! cf. [DISTANCES.md](../../../docs/DISTANCES.md)
//!
//! NOTE: This is like 1s of runtime compared to the *several minutes* of the previous version.
//!

use eyre::Result;

use crate::cmds::{HomeStats, Stats};
use crate::config::Context;

/// Update the given table with calculus of the distance between a drone and its operator
///
/// `dist_2d` has been updated to use `ST_Distance_Spheroid()`
/// `dist_3d` has been updated to use `dist_2d`.
///
#[tracing::instrument(skip(ctx))]
pub fn home_calculation(ctx: &Context) -> Result<Stats> {
    let dbh = ctx.db();

    let mut stats = HomeStats::new();

    // Simple update now.
    //
    let sql_update = r##"
UPDATE
  drones
SET
  home_distance_2d = 
    dist_2d(longitude, latitude, home_lon, home_lat),
  home_distance_3d = 
    dist_3d(longitude, latitude, altitude, home_lon, home_lat, home_height)
"##;

    let count = dbh.execute(sql_update, [])?;
    stats.distances = count;

    Ok(Stats::Home(stats))
}
