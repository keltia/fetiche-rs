//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!

use clap::Parser;
use duckdb::Connection;
use eyre::Result;
use tracing::info;

use crate::config::Context;

#[derive(Debug, Parser)]
pub struct SetupOpts {
    /// Add only macros.
    #[clap(short = 'M', long)]
    pub macros: bool,
    /// Add columns to table drones.
    #[clap(short = 'C', long)]
    pub columns: bool,
    /// Create encounters table
    #[clap(short = 'E', long)]
    pub encounters: bool,
    /// Everything.
    #[clap(short = 'a', long)]
    pub all: bool,
}

#[tracing::instrument(skip(dbh))]
fn add_macros(dbh: &Connection) -> Result<()> {
    info!("Adding macros.");

    let r = r##"
CREATE MACRO nm_to_deg(nm) AS
  nm * 1.852 / 111111.11;
CREATE MACRO deg_to_m(deg) AS
  deg * 111111.11;
CREATE MACRO m_to_deg(m) AS
  m / 111111.11;
CREATE MACRO dist_2d(px, py, dx, dy) AS
  ST_Distance_Spheroid(ST_Point(px, py), ST_Point(dx, dy));
CREATE MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  sqrt(pow(dist_2d(px, py, dx, dy), 2) + pow((pz - dz), 2));
CREATE MACRO encounter(tm, journey, id) AS
  printf('%04d%02d%02d_%d_%d', datepart('year', CAST(tm AS DATE)), datepart('month', CAST(tm AS DATE)), datepart('day', CAST(tm AS DATE)), journey, id);
    "##;

    Ok(dbh.execute_batch(r)?)
}

#[tracing::instrument(skip(dbh))]
fn remove_macros(dbh: &Connection) -> Result<()> {
    info!("Removing macros.");

    let r = r##"
DROP MACRO dist_2d;
DROP MACRO dist_3d;
DROP MACRO nm_to_deg;
DROP MACRO deg_to_m;
DROP MACRO m_to_deg;
DROP MACRO encounter;
    "##;

    Ok(dbh.execute_batch(r)?)
}

/// Create the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
fn add_encounters_table(dbh: &Connection) -> Result<()> {
    info!("Adding encounters table.");

    let sq = r##"
DROP SEQUENCE IF EXISTS id_encounter;
CREATE SEQUENCE id_encounter;
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
  distancelat FLOAT,
  distancevert FLOAT,
  PRIMARY KEY (dt, journey)
)
    "##;

    if dbh
        .execute("SELECT id FROM encounters LIMIT 1", [])
        .is_err()
    {
        // Create sequence & table.
        //
        dbh.execute_batch(sq)?;
    }
    Ok(())
}

/// Remove the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(dbh))]
fn drop_encounters_table(dbh: &Connection) -> Result<()> {
    info!("Removing encounters table.");

    let sq = r##"
DROP SEQUENCE IF EXISTS id_encounter;
DROP TABLE IF EXISTS encounters;
    "##;

    dbh.execute_batch(sq)?;
    Ok(())
}

#[tracing::instrument(skip(ctx))]
pub fn setup_acute_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db();
    if opts.all {
        add_macros(&dbh)?;
        add_encounters_table(&dbh)?;
    } else {
        if opts.macros {
            add_macros(&dbh)?;
        }
        if opts.encounters {
            add_encounters_table(&dbh)?;
        }
    }
    Ok(())
}

#[tracing::instrument(skip(ctx))]
pub fn cleanup_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dbh = ctx.db();
    if opts.all {
        remove_macros(&dbh)?;
        drop_encounters_table(&dbh)?;
    } else {
        if opts.macros {
            remove_macros(&dbh)?;
        }
        if opts.columns {}
        if opts.encounters {
            drop_encounters_table(&dbh)?;
        }
    }
    remove_macros(&dbh)?;

    Ok(())
}
