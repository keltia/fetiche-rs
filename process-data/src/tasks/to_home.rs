//! for all drone points:
//!     convert drone location into Point_2D
//!     convert home location into Point_2D
//!     compute distance and store in table
//!     compute 3d distance
//!     insert both in base
//!
//! We also need the following macros defined:
//! `dist_2d`, `dist_3d` and `deg_to_m`
//!
//! cf. [DISTANCES.md](../../../docs/DISTANCES.md)
//!
//! NOTE: This is like 1s of runtime compared to the *several minutes* of the previous version.
//!

use duckdb::Connection;
use eyre::Result;

/// Update the given table with calculus of the distance between a drone and its operator
///
pub fn home_calculation(dbh: &Connection) -> Result<()> {
    let dbh = dbh.try_clone()?;

    // Simple update now.
    //
    let sql_update = r##"
UPDATE
  drones
SET
  home_distance_2d = 
    deg_to_m(dist_2d(longitude, latitude, home_lon, home_lat)),
  home_distance_3d = 
    deg_to_m(dist_3d(longitude, latitude, m_to_deg(altitude), home_lon, home_lat, m_to_deg(home_height)))
"##;

    let _ = dbh.execute(sql_update, [])?;
    Ok(())
}