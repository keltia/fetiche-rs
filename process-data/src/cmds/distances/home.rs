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

use eyre::{eyre, Result};
use tokio::time::Instant;

use crate::cmds::{HomeStats, Stats};
use crate::config::Context;

/// Update the given table with calculus of the distance between a drone and its operator
///
/// `dist_2d` has been updated to use `ST_Distance_Spheroid()`
/// `dist_3d` has been updated to use `dist_2d`.
///
#[tracing::instrument(skip(ctx))]
pub fn home_calculation(ctx: &Context) -> Result<Stats> {
    return Err(eyre!("DEPRECATED."));

    let dbh = ctx.db();

    let mut stats = HomeStats::new();

    let start = Instant::now();

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
 WHERE
   home_distance_2d IS NULL OR
   home_distance_2d = 0
"##;

    let count = dbh.execute(sql_update, [])?;
    stats.updated = count;

    let count = dbh.query_row("SELECT COUNT(*) FROM drones", [], |row| {
        let r: usize = row.get_unwrap(0);
        Ok(r)
    })?;
    stats.total = count;
    stats.time = (Instant::now() - start).as_millis();

    eprintln!("{}", stats);

    Ok(Stats::Home(stats))
}
