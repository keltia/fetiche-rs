//! Setup needed tables in ClickHouse.
//!

use crate::cmds::DBVars;
use crate::make_query;
use crate::runtime::Context;

// ----- Core tables

/// Create the raw ADS-B positions table, part of the core.
///
/// This is the UTC-only version.
///
/// ### Details
/// This function creates the `airplanes_raw` table in the ClickHouse database.
/// The table is designed to store raw ADS-B (Automatic Dependent Surveillance-Broadcast)
/// position data received from aircraft transponders.
///
/// The table includes the following fields:
/// - `Site`: Site identifier where the data was received (INT)
/// - `EmitterCategory`: Category of the emitting aircraft (INT, default: 3)
/// - `GBS`: Ground-Based Station indicator (INT)
/// - `ModeA`: Transponder Mode A code (VARCHAR)
/// - `TimeRecPosition`: Timestamp when the position was recorded (DATETIME64 with millisecond precision in UTC)
/// - `AircraftAddress`: Unique ICAO 24-bit aircraft address (VARCHAR)
/// - `Latitude`: Aircraft latitude in decimal degrees (DOUBLE)
/// - `Longitude`: Aircraft longitude in decimal degrees (DOUBLE)
/// - `GeometricAltitude`: Geometric altitude above WGS84 ellipsoid (DOUBLE)
/// - `FlightLevel`: Barometric altitude/flight level (DOUBLE)
/// - `BarometricVerticalRate`: Rate of climb/descent from barometric altitude (VARCHAR)
/// - `GeoVertRateExceeded`: Flag indicating if geometric vertical rate exceeded limits (VARCHAR)
/// - `GeometricVerticalRate`: Rate of climb/descent from geometric altitude (VARCHAR)
/// - `GroundSpeed`: Speed over ground (DOUBLE)
/// - `TrackAngle`: Ground track angle in degrees (DOUBLE)
/// - `Callsign`: Aircraft callsign/flight number (VARCHAR)
/// - `AircraftStopped`: Flag indicating if aircraft is stopped on ground (VARCHAR)
/// - `GroundTrackValid`: Flag indicating validity of ground track data (VARCHAR)
/// - `GroundHeadingProvided`: Flag indicating if ground heading is provided (VARCHAR)
/// - `MagneticNorth`: Flag indicating if heading is relative to magnetic north (VARCHAR)
/// - `SurfaceGroundSpeed`: Ground speed when on surface (DOUBLE)
/// - `SurfaceGroundTrack`: Ground track when on surface (DOUBLE)
///
/// The table uses the MergeTree engine with a primary key on `(TimeRecPosition, AircraftAddress)`
/// to optimize queries filtering by time and aircraft address.
///
/// ### Errors
/// Returns an error if the table cannot be created. Possible causes include:
/// - Database connection issues
/// - Insufficient privileges to create tables in the database
/// - SQL syntax or schema errors
/// - Database name variables not properly configured in the context
///
/// ### References
/// - ClickHouse MergeTree engine documentation
/// - ADS-B message format specifications
///
#[tracing::instrument(skip(ctx))]
pub async fn create_airplanes_raw_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {planedb}.airplanes_raw (
    Site                   INT,
    EmitterCategory        INT DEFAULT 3,
    GBS                    INT,
    ModeA                  VARCHAR,
    TimeRecPosition        DATETIME64(3, 'UTC'),
    AircraftAddress        VARCHAR,
    Latitude               DOUBLE,
    Longitude              DOUBLE,
    GeometricAltitude      DOUBLE,
    FlightLevel            DOUBLE,
    BarometricVerticalRate VARCHAR,
    GeoVertRateExceeded    VARCHAR,
    GeometricVerticalRate  VARCHAR,
    GroundSpeed            DOUBLE,
    TrackAngle             DOUBLE,
    Callsign               VARCHAR,
    AircraftStopped        VARCHAR,
    GroundTrackValid       VARCHAR,
    GroundHeadingProvided  VARCHAR,
    MagneticNorth          VARCHAR,
    SurfaceGroundSpeed     DOUBLE,
    SurfaceGroundTrack     DOUBLE
)
ENGINE = MergeTree
PRIMARY KEY (TimeRecPosition, AircraftAddress)
COMMENT 'Table for raw ADS-B positions.'
    "##,
        dbvars
    );

    Ok(dbh.execute(&r).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_airplanes_raw_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(r##"DROP TABLE IF EXISTS {planedb}.airplanes_raw"##, dbvars);

    Ok(dbh.execute(&r).await?)
}

// -----

/// Create the raw drones positions table, part of the core.
///
/// This is the UTC-only version.
///
/// ### Details
/// This function creates the `drones_raw` table in the ClickHouse database.
/// The table is designed to store raw position data received from drone tracking systems
/// across multiple sites and journeys.
///
/// The table includes the following fields:
/// - `journey`: Journey identifier for the drone flight (INT)
/// - `ident`: Unique identifier for the drone (VARCHAR)
/// - `model`: Drone model information (VARCHAR)
/// - `source`: Source of the tracking data (VARCHAR)
/// - `location`: Location identifier (INT)
/// - `timestamp`: Timestamp when the position was recorded (DATETIME in UTC)
/// - `latitude`: Drone latitude in decimal degrees (DOUBLE)
/// - `longitude`: Drone longitude in decimal degrees (DOUBLE)
/// - `altitude`: Drone altitude (INT)
/// - `elevation`: Elevation above ground level (INT)
/// - `gps`: GPS signal quality indicator (INT)
/// - `rssi`: Received Signal Strength Indication (INT)
/// - `home_lat`: Home location latitude in decimal degrees (DOUBLE)
/// - `home_lon`: Home location longitude in decimal degrees (DOUBLE)
/// - `home_height`: Home location height (INT)
/// - `speed`: Drone speed (INT)
/// - `heading`: Drone heading in degrees (INT)
/// - `station_name`: Name of the receiving station (VARCHAR)
/// - `station_latitude`: Station latitude in decimal degrees (DOUBLE)
/// - `station_longitude`: Station longitude in decimal degrees (DOUBLE)
///
/// The table uses the MergeTree engine with a primary key on `(journey, timestamp)`
/// and is ordered by the same fields with an index granularity of 8192 to optimize
/// queries filtering by journey and time.
///
/// ### Errors
/// Returns an error if the table cannot be created. Possible causes include:
/// - Database connection issues
/// - Insufficient privileges to create tables in the database
/// - SQL syntax or schema errors
/// - Database name variables not properly configured in the context
///
/// ### References
/// - ClickHouse MergeTree engine documentation
/// - Drone tracking data format specifications
///
#[tracing::instrument(skip(ctx))]
pub async fn create_drones_raw_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {drobedb}.drones_raw               
(                                               
    journey INT,                            
    ident VARCHAR,                             
    model VARCHAR,                             
    source VARCHAR,                            
    location INT,                           
    timestamp DateTime('UTC'),                       
    latitude DOUBLE,                         
    longitude DOUBLE,                        
    altitude INT,                           
    elevation INT,                          
    gps INT,                                
    rssi INT,                               
    home_lat DOUBLE,                         
    home_lon DOUBLE,                         
    home_height INT,                        
    speed INT,                              
    heading INT,                            
    station_name VARCHAR,                      
    station_latitude DOUBLE,                 
    station_longitude DOUBLE                 
)                                               
ENGINE = MergeTree                              
PRIMARY KEY (journey, timestamp)                
ORDER BY (journey, timestamp)                   
SETTINGS index_granularity = 8192               
COMMENT 'Raw positions for drones on all sites.'
        "##,
        dbvars
    );

    Ok(dbh.execute(&r).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_drones_raw_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let r = make_query!(r##"DROP TABLE IF EXISTS {drobedb}.drones_raw"##, dbvars);

    Ok(dbh.execute(&r).await?)
}

// -----

/// Create the `encounters` table to store short air-prox points
///
/// ### Details
///
/// This function creates the `encounters` table (`airplane_prox`) in the database.
/// The table is structured to store proximity data related to drone and airplane encounters
/// within a specified distance threshold.
///
/// - `site`: Represents the site ID where the encounter occurs.
/// - `sitename`: Name of the site where the encounter occurs.
/// - `en_id`: Unique identifier for the encounter.
/// - `time`: Timestamp indicating when the encounter occurred.
/// - `journey`: Identifier for the journey or flight path of the drone.
/// - `drone_id`: The unique identifier for the drone.
/// - `model`: Drone model information.
/// - `drone_lat`: Latitude of the drone.
/// - `drone_lon`: Longitude of the drone.
/// - `drone_alt_m`: Altitude of the drone in meters.
/// - `drone_height_m`: Drone height in meters above ground level.
/// - `prox_callsign`: Callsign of the nearby airplane.
/// - `prox_id`: Unique identifier of the airplane.
/// - `prox_lat`: Latitude of the airplane.
/// - `prox_lon`: Longitude of the airplane.
/// - `prox_alt_m`: Altitude of the airplane in meters.
/// - `prox_mode_a`: Squawk code of the aircraft.
/// - `prox_ecat`: Category of the airplane (e.g., GBS, UAS, etc. EmitterCategory from ADS-B).
/// - `distance_slant_m`: Slant distance in meters between the drone and airplane.
/// - `distance_hor_m`: Horizontal distance in meters between the drone and airplane.
/// - `distance_vert_m`: Vertical distance in meters between the drone and airplane.
/// - `distance_home_m`: Distance in meters between the drone and its home location.
///
/// This command creates the `airplane_prox` table in the database to store the
/// described data points for encounters.
///
/// ### Errors
///
/// Returns an error if the table cannot be created. Possible causes include:
/// - Database connection issues.
/// - Insufficient privileges to create tables in the database.
/// - SQL syntax or schema errors.
///
/// ### References
///
/// - ClickHouse documentation for managing tables.
#[tracing::instrument(skip(ctx))]
pub async fn add_encounters_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let sq = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {workdb}.airplane_prox (
  site_id          INT,
  sitename         VARCHAR,
  en_id            VARCHAR,
  time             TIMESTAMP,
  journey          INT,
  drone_id         VARCHAR,
  model            VARCHAR,
  drone_lat        FLOAT,
  drone_lon        FLOAT,
  drone_alt_m      FLOAT,
  drone_height_m   FLOAT,
  prox_callsign    VARCHAR,
  prox_id          VARCHAR,
  prox_lat         FLOAT,
  prox_lon         FLOAT,
  prox_alt_m       FLOAT,
  prox_mode_a      VARCHAR,
  prox_ecat        INT,
  distance_slant_m INT,
  distance_hor_m   INT,
  distance_vert_m  INT,
  distance_home_m  INT,
  station_name     VARCHAR
)
    ENGINE = ReplacingMergeTree PRIMARY KEY (time, journey)
    COMMENT 'Store all plane-drone encounters with less then 1nm distance.';
    "##,
        dbvars
    );

    Ok(dbh.execute(&sq).await?)
}

/// Remove the `encounters` table to store short air-prox points
///
#[tracing::instrument(skip(ctx))]
pub async fn drop_encounters_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let sq = make_query!(r##"DROP TABLE IF EXISTS {workdb}.airplane_prox"##, dbvars);

    Ok(dbh.execute(&sq).await?)
}

// ----- Record-related table

#[tracing::instrument(skip(ctx))]
pub async fn add_daily_stats_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let crt = make_query!(
        r##"
CREATE TABLE IF NOT EXISTS {workdb}.daily_stats (
  day DATE,
  site_id INT,
  site_name VARCHAR,
  status INT NOT NULL,
  stats VARCHAR,
  comment VARCHAR,
)
ENGINE = ReplacingMergeTree PRIMARY KEY (day, site_name)
COMMENT 'Records the run history for all sites every day.';
    "##,
        dbvars
    );

    Ok(dbh.execute(&crt).await?)
}

#[tracing::instrument(skip(ctx))]
pub async fn drop_daily_stats_table(ctx: &Context) -> eyre::Result<()> {
    let dbh = ctx.db().await;
    let dbvars = DBVars::from_ctx(ctx);

    let crt = make_query!(
        r##"DROP TABLE {workdb}.daily_stats IF EXISTS {workdb}.daily_stats"##,
        dbvars
    );

    Ok(dbh.execute(&crt).await?)
}
