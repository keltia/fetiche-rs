//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!

use duckdb::Connection;
use eyre::Result;

fn add_macros(dbh: &Connection) -> Result<()> {
    let r = r##"
CREATE MACRO dist_2d(px, py, dx, dy) AS
  sqrt(pow((px - dx),2) + pow((py - dy), 2));
CREATE MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  sqrt(pow((px - dx),2) + pow((py - dy), 2) + pow((pz - dz), 2));
CREATE MACRO nm_to_deg(nm) AS
  nm * 1.852 / 111111.11;
CREATE MACRO deg_to_m(deg) AS
  deg * 111111.11;
CREATE MACRO m_to_deg(m) AS
  m / 111111.11;
CREATE MACRO encounter(tm, journey, id) AS
  printf('%04d%02d%02d_%d_%d', datepart('year', CAST(tm AS DATE)), datepart('month', CAST(tm AS DATE)), datepart('day', CAST(tm AS DATE)), journey, id);
    "##;

    Ok(dbh.execute_batch(r)?)
}

fn add_columns_to_drones(dbh: &Connection) -> Result<()> {
    let r = r##"
ALTER TABLE drones
  ADD COLUMN home_distance_2d FLOAT;
ALTER TABLE drones
  ADD COLUMN home_distance_3d FLOAT;
    "##;

    // Assume that if home_distance_2d doesn't exist, then home_distance_3d doesn't either.
    //
    match dbh.execute("SELECT home_distance_2d FROM drones LIMIT 1", []) {
        Ok(_) => (),
        Err(_) => {
            let _ = dbh.execute(r, [])?;
        }
    }
    Ok(())
}

/// Create the `encounters` table to store short air-prox points
///
fn add_encounters_table(dbh: &Connection) -> Result<()> {
    let sq = r##"
DROP SEQUENCE IF EXISTS id_encounter;
CREATE SEQUENCE id_encounter
    "##;

    let r = r##"
CREATE TABLE encounters (
  id INT DEFAULT nextval('id_encounter'),
  en_id VARCHAR,
  dt INT,    
  time VARCHAR,
  site VARCHAR,
  journey INT, 
  drone_id VARCHAR,
  model VARCHAR,
  callsign VARCHAR, 
  addr VARCHAR, 
  distance FLOAT,
  PRIMARY KEY (dt, journey)
)
    "##;

    match dbh.execute("SELECT id FROM encounters LIMIT 1", []) {
        Ok(_) => (),
        Err(_) => {
            // create sequence
            //
            let _ = dbh.execute(sq, [])?;

            // create table
            //
            let _ = dbh.execute(r, [])?;
        }
    }
    Ok(())
}

fn load_extensions(dbh: &Connection) -> Result<()> {
    // Load our extensions
    //
    let _ = dbh.execute("LOAD spatial", [])?;
    Ok(())
}

pub fn setup_acute_environment(dbh: &Connection) -> Result<()> {
    let _ = load_extensions(dbh)?;
    let _ = add_macros(dbh)?;
    let _ = add_columns_to_drones(dbh)?;
    let _ = add_encounters_table(dbh)?;

    Ok(())
}
