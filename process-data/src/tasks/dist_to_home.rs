//! for all drone points:
//!     convert drone location into Point_2D
//!     convert home location into Point_2D
//!     compute distance and store in table
//!

use duckdb::{params, Connection};
use eyre::Result;
use geo::{point, *};

/// Update the given table with calculus of the distance between a drone and its operator
///
pub fn distance_calculation(dbh: &Connection) -> Result<()> {
    let mut dbh = dbh.try_clone()?;

    let tx = dbh.transaction()?;
    let mut stmt = tx.prepare(
        r##"
SELECT
  time, journey, latitude, longitude, home_lat, home_lon, home_distance
FROM
  drones
WHERE
  home_distance IS NULL
    "##,
    )?;

    // Get all incomplete records
    //
    let list_items = stmt.query_map([], |row| {
        let time: u64 = row.get_unwrap(0);
        let journey: u32 = row.get_unwrap(1);
        let latitude: f64 = row.get_unwrap(2);
        let longitude: f64 = row.get_unwrap(3);
        let home_lat: f64 = row.get(4).unwrap_or(0.);
        let home_lon: f64 = row.get(5).unwrap_or(0.);

        let drone = point!(x: longitude, y: latitude);
        let home = point!(x: home_lon, y: home_lat);
        let dist = drone.geodesic_distance(&home);
        Ok((time, journey, dist))
    })?;

    let sql_update = r##"
UPDATE
  drones
SET
  home_distance = ?
WHERE
  time = ? AND journey = ?
"##;

    let mut stmt = tx.prepare(sql_update)?;
    list_items.for_each(|row| {
        match row {
            Ok((dist, time, journey)) => {
                let _ = stmt.execute(params![dist, time, journey]);
                eprint!(".");
            }
            Err(_) => (),
        };
    });
    let _ = tx.commit()?;
    Ok(())
}
