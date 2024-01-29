//! Module computing the distance from a drone to the various planes around
//!

use chrono::Datelike;
use duckdb::Connection;
use eyre::{eyre, Result};

use crate::cli::PlanesOpts;
use crate::tasks::to_planes::location::load_locations;

mod location;

/// Select a list of airplanes positions we will consider for distance calculations
///
/// - 1st criteria date and time (unit is a given day)
/// - define a bounding box around a specific site (default is 70nm) and use it as a filter
///
fn select_planes(dbh: &Connection, opts: &PlanesOpts) -> Result<()> {
    let day = opts.date.day();
    let month = opts.date.month();
    let year = opts.date.year();

    // All flights for a given day in a table
    //
    // $1 = site
    // $2 = year
    // $3 = month
    // $4 = start of day
    // $5 = end of day
    // $6 = lon of site
    // $7 = lat of site
    // $8 = distance in degrees (== dist(nm) /  60)   1 deg ~ 60 nm
    //
    let r1 = r##"
CREATE TEMP TABLE today AS
SELECT
  "073.TimeRecPosition" AS time,
  "080.AircraftAddress" AS ident,
  "131.Longitude" AS px,
  "131.Latitude" AS py,
  CAST("140.GeometricAltitude" AS DOUBLE) AS pz,
  "170.Callsign" AS callsign
FROM
  read_parquet('../adsb/**/*.parquet')
WHERE
  site=? AND
  year=? AND
  month=? AND
  time >= ? AND
  time <= ? AND
    ST_DWithin(
        ST_point(?, ?),
        ST_Point(px, py),
        ?
    )
    "##;

    let mut stmt = dbh.prepare(r1)?;

    Ok(())
}

fn select_drones(dbh: &Connection, opts: &PlanesOpts) -> Result<()> {
    // All drone points for the same day
    //
    // $1 = year
    // $2 = month
    // $3 = day
    // $4,$5 = (lon,lat) site
    // $6 = distance in degrees
    //
    let r2 = r##"
CREATE TEMP TABLE candidates AS
SELECT *
FROM drones
WHERE
  to_timestamp(time) <= make_timestamp(?,?,? + 1,0,0,0.0) AND
  to_timestamp(time) >= make_timestamp(?,?,?,0,0,0.0) AND
  ST_DWithin(
    ST_point(?, ?), ST_Point(longitude, latitude), ?
  )
    "##;

    Ok(())
}

fn find_close(dbh: &Connection, opts: &PlanesOpts) -> Result<()> {
    // Select planes points that are in temporal and geospatial proximity +- 3 nm ~ 0.05 deg and
    // altitude diff is less than 3 nm.
    //
    // $1,$2 = lon,lat of site
    // $3 = timestamp of drone point
    //
    let r = r##"
CREATE TEMP TABLE close AS
SELECT
  t.addr,
  t.callsign,
  t.time AS pt,
  t.px AS px,
  t.py AS py,
  t.pz as pz,
  c.ident,
  c.model,
  c.time AS dt,
  c.longitude AS dx,
  c.latitude AS dy,
  c.altitude AS dz,
  st_distance(st_point(dx,dy),st_point(px,py)) AS dist2d,
  @(pz - dz) AS diff_alt
FROM
  today AS t,
  candidates AS c
WHERE 
  pt > to_timestamp(dt-2) AND 
  pt < to_timestamp(dt+2) AND
  (@(pz - dz) < 5500)  
    "##;
}

pub fn planes_calculation(dbh: &Connection, opts: PlanesOpts) -> Result<()> {
    // Load locations
    //
    let list = load_locations(None)?;
    let day = opts.date;

    // Load parameters
    //
    let name = opts.location.clone();
    let current = if list.get(&opts.location).is_none() {
        return Err(eyre!("Unknown location: {}", opts.location));
    } else {
        list.get(&opts.location).unwrap()
    };

    let list_planes: Vec<_> = select_planes(&dbh, &opts)?;
    if list_planes.is_empty() {
        return Err(eyre!("No planes found around {}.", &name));
    }

    let list_drones: Vec<_> = select_drones(&dbh, &opts)?;

    Ok(())
}
