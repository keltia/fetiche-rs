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
use geo::{point, *};
use rust_3d::Point3D;

/// 1 deg = 59.9952 nm or 111.1111 km
const R: f64 = 111_111.11;

/// Calculate the 3D distance from home to drone in METERS
///
/// 2D dist is √(Δx𝟤 + Δy𝟤)
/// 3D dist is √(Δx𝟤 + Δy𝟤 + Δz𝟤)
///
fn calculate_3d_distance(p1: &Point3D, p2: &Point3D) -> f64 {
    // Normalise in degrees
    //
    let drone_elev = (p1.z - p2.z) / R;

    // 3D distance
    //
    let tmp = (drone_elev.powi(2) + (p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt();

    // Return in METERS
    //
    tmp * R
}

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

        let drone = point!(x: longitude, y: latitude);
        let home = point!(x: home_lon, y: home_lat);

        // 2D projected distance in METERS
        //
        let dist2d = drone.geodesic_distance(&home);

        // 3D distance
        //
        // Into degrees
        //
        let altitude = altitude / R;
        let home_height = home_height / R;

        let drone = Point3D::new(longitude, latitude, altitude);
        let home = Point3D::new(home_lon, home_lat, home_height);

        // In METERS
        //
        let dist3d = calculate_3d_distance(&drone, &home);

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
