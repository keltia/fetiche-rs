//! for all drone points:
//!     convert drone location into Point_2D
//!     convert home location into Point_2D
//!     compute distance and store in table
//!     compute 3d distance
//!     insert both in base
//!
//! Assume table was modified as follows:
//! ```sql
//! ALTER TABLE drones ADD COLUMN home_distance_2d FLOAT;
//! ALTER TABLE drones ADD COLUMN home_distance_3d FLOAT;
//! ```
//!

use duckdb::{params, Connection};
use eyre::Result;
use rust_3d::Point2D;

/// 1 deg = 59.9952 nm or 111.1111 km
const R: f64 = 40_000_000. / 360.;

/// Update the given table with calculus of the distance between a drone and its operator
///
pub fn home_calculation(dbh: &Connection) -> Result<()> {
    let mut dbh = dbh.try_clone()?;

    let tx = dbh.transaction()?;
    let mut stmt = tx.prepare(
        r##"
SELECT
  time, journey, latitude, longitude, altitude, home_lat, home_lon, home_height, home_distance_2d, home_distance_3d
FROM
  drones
WHERE
  home_distance_2d IS NULL OR home_distance_3d IS NULL
    "##,
    )?;

    // Get all incomplete records
    //
    let list_items = stmt.query_map([], |row| {
        let time: u64 = row.get_unwrap(0);
        let journey: u32 = row.get_unwrap(1);
        let latitude: f64 = row.get_unwrap(2);
        let longitude: f64 = row.get_unwrap(3);
        let altitude: f64 = row.get(4).unwrap_or(0.);
        let home_lat: f64 = row.get(5).unwrap_or(0.);
        let home_lon: f64 = row.get(6).unwrap_or(0.);
        let home_height: f64 = row.get(7).unwrap_or(0.);

        let drone = Point2D::new(longitude, latitude);
        let home = Point2D::new(home_lon, home_lat);

        // 2D projected distance in METERS
        //
        // 2D dist is √(Δx𝟤 + Δy𝟤), cache Δx𝟤 + Δy𝟤 for later
        //
        let a2b2 = (drone.x - home.x).powi(2) + (drone.y - home.y).powi(2);

        // 3D distance
        //
        // Into degrees
        //
        let altitude = altitude / R;
        let home_height = home_height / R;

        // Calculate the 3D distance from home to drone in METERS
        //
        // 3D dist is √(Δx𝟤 + Δy𝟤 + Δz𝟤)
        //
        let dist3d = (a2b2 + (altitude - home_height).powi(2)).sqrt();

        // Transform into meters
        //
        let dist2d = a2b2.sqrt() * R;
        let dist3d = dist3d * R;

        Ok((time, journey, dist2d, dist3d))
    })?;

    let sql_update = r##"
UPDATE
  drones
SET
  home_distance_2d = ?, home_distance_3d = ?
WHERE
  time = ? AND journey = ?
"##;

    let mut stmt = tx.prepare(sql_update)?;
    let mut p = progress::SpinningCircle::new();
    p.set_job_title("updating");
    list_items.for_each(|row| {
        match row {
            Ok((time, journey, dist2d, dist3d)) => {
                let _ = stmt.execute(params![dist2d, dist3d, time, journey]);
                p.tick();
            }
            Err(_) => (),
        };
    });
    p.jobs_done();
    let _ = tx.commit()?;
    Ok(())
}
