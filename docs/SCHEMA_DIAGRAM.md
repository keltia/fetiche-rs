# ClickHouse Database Schema & Relationship Diagram

This document provides a comprehensive overview and ASCII diagrams of all database tables, views, materialized views, and their relationships in the **ACUTE / Fetiche-CH** ClickHouse database schema as defined in `process-data/src/cmds/setup/`.

---

## 1. High-Level Architecture & Relationship Diagram

```
 +-----------------------------------------------------------------------------+
 |                              METADATA TABLES                                |
 |                                                                             |
 |  +---------------+         +-----------------+         +-----------------+  |
 |  |     sites     |         |  installations  |         |    antennas     |  |
 |  +---------------+         +-----------------+         +-----------------+  |
 |  | PK  id        |<--------| FK  site_id     |         | PK  id          |  |
 |  |     name      |         | FK  antenna_id  |-------->|     type        |  |
 |  |     code      |         |     start_at    |         |     name        |  |
 |  |     latitude  |         |     end_at      |         |     owned       |  |
 |  |     longitude |         +--------+--------+         +--------+--------+  |
 |  |     timezone  |                  |                           |           |
 |  +-------+-------+                  +-------------+-------------+           |
 +----------|----------------------------------------|-------------------------+
            |                                        |
            |                                        v
            |                         +-----------------------------+
            |                         |         DEPLOYMENTS         |
            |                         |            VIEWS            |
            |                         +-----------------------------+
            |                         | deployments                 |
            |                         | pbi_deployments             |
            |                         +--------------+--------------+
            |                                        |
 +----------|----------------------------------------|-------------------------+
 |          |               CORE SURVEILLANCE DATA   |                         |
 |          |                                        |                         |
 |  +-------v---------+               +--------------v--+                      |
 |  |  airplanes_raw  |               |   drones_raw    |                      |
 |  +-----------------+               +-----------------+                      |
 |  | PK(Time,Address)|               | PK(journey,time)|                      |
 |  +--------+--------+               +--------+--------+                      |
 |           |                                 |   |                           |
 |           v                                 |   v                           |
 |  +-----------------+                        |  +-----------------+          |
 |  |    airplanes    | (VIEW)                 |  |     drones      | (MAT VIEW|
 |  +-----------------+                        |  +-----------------+          |
 |                                             |                               |
 |                                             v                               |
 |                                    +-----------------+                      |
 |                                    |   pbi_drones    | (VIEW)               |
 |                                    +-----------------+                      |
 +---------------------------------------------|-------------------------------+
                                               |
 +---------------------------------------------v-------------------------------+
 |                       ENCOUNTER & PROXIMITY ANALYSIS                        |
 |                                                                             |
 |                                    +-----------------+                      |
 |                                    |  airplane_prox  | (TABLE)              |
 |                                    +-----------------+                      |
 |                                    | PK(time,journey)|                      |
 |                                    +---+-----------+-+                      |
 |                                        |           |                        |
 |                     +------------------+           +--------------+         |
 |                     |                                             |         |
 |                     v                                             v         |
 |            +-----------------+                           +-----------------+|
 |            | airprox_summary | (VIEW)                    | pbi_encounters  ||(MAT VIEW)
 |            +-----------------+                           +--------+--------+|
 |                     |                                             |         |
 |                     +----------------------+----------------------+         |
 |                                            |                                |
 |                                            v                                |
 |                             +----------------------------+                  |
 |                             |   pbi_encounters_summary   | (MAT VIEW)       |
 |                             +----------------------------+                  |
 +-----------------------------------------------------------------------------+
 |                             OPERATIONAL METRICS                             |
 |                                                                             |
 |                             +----------------------------+                  |
 |                             |        daily_stats         | (TABLE)          |
 |                             +----------------------------+                  |
 |                             | PK(day, site_name)         |                  |
 |                             +----------------------------+                  |
 +-----------------------------------------------------------------------------+
```

---

## 2. Detailed Table & View Schema Diagram

```
===================================================================================================================
                                              METADATA TABLES
===================================================================================================================

+-----------------------------------+             +----------------------------------+
|               sites               |             |             antennas             |
+-----------------------------------+             +----------------------------------+
| PK  id           INT              |             | PK  id          INT              |
|     name         VARCHAR NOT NULL |<----+       |     type        VARCHAR NOT NULL |<----+
|     code         VARCHAR          |     |       |     name        VARCHAR NOT NULL |     |
|     basename     VARCHAR NOT NULL |     |       |     owned       BOOLEAN          |     |
|     latitude     Float64 NOT NULL |     |       |     description VARCHAR          |     |
|     longitude    Float64 NOT NULL |     |       +----------------------------------+     |
|     ref_altitude INT NOT NULL     |     |       Engine: MergeTree                        |
|     timezone     VARCHAR NOT NULL |     |       Primary Key: (id)                        |
|     offset       INT NOT NULL     |     |                                                |
+-----------------------------------+     |                                                |
Engine: MergeTree                         |                                                |
Primary Key: (id)                         |                                                |
                                          |                                                |
                                          |       +----------------------------------+     |
                                          |       |          installations           |     |
                                          |       +----------------------------------+     |
                                          |       | PK  id         INT               |     |
                                          +-------| FK  site_id    INT NOT NULL      |     |
                                                  | FK  antenna_id INT NOT NULL      |-----+
                                                  |     start_at   DateTime('UTC')   |
                                                  |     end_at     DateTime('UTC')   |
                                                  |     comment    VARCHAR           |
                                                  +----------------------------------+
                                                  Engine: MergeTree
                                                  Primary Key: (id)
                                                  Order By: (id)

===================================================================================================================
                                              DEPLOYMENT VIEWS
===================================================================================================================

+-------------------------------------------------+   +-------------------------------------------------+
|                   deployments                   |   |                 pbi_deployments                 |
|                     (VIEW)                      |   |                     (VIEW)                      |
+-------------------------------------------------+   +-------------------------------------------------+
| Sources: installations i, antennas a, sites s   |   | Sources: installations i, antennas a, sites s   |
| Join: i.antenna_id = a.id AND s.id = i.site_id  |   | Join: i.antenna_id = a.id AND s.id = i.site_id  |
+-------------------------------------------------+   +-------------------------------------------------+
| install_id   INT          (from i.id)           |   | installation_id INT         (from i.id)         |
| start_at     DateTime     (from i.start_at)     |   | start_at        DateTime    (from i.start_at)   |
| end_at       DateTime     (from i.end_at)       |   | end_at          DateTime    (from i.end_at)     |
| type         VARCHAR      (from a.type)         |   | type            VARCHAR     (from a.type)       |
| antenna_name VARCHAR      (from a.name)         |   | antenna_name    VARCHAR     (from a.name)       |
| site_name    VARCHAR      (from s.name)         |   | sitename        VARCHAR     (from s.name)       |
| site_id      INT          (from s.id)           |   | site_id         INT         (from s.id)         |
| timezone     VARCHAR      (from s.timezone)     |   | timezone        INT         (from s.offset)     |
+-------------------------------------------------+   | tzname          VARCHAR     (from s.timezone)   |
                                                      | latitude        Float64     (from s.latitude)   |
                                                      | longitude       Float64     (from s.longitude)  |
                                                      | ref_altitude    INT         (from s.ref_alt)    |
                                                      +-------------------------------------------------+

===================================================================================================================
                                         CORE SURVEILLANCE TABLES & VIEWS
===================================================================================================================

+-------------------------------------------------+   +-------------------------------------------------+
|                  airplanes_raw                  |   |                   drones_raw                    |
|                     (TABLE)                     |   |                     (TABLE)                     |
+-------------------------------------------------+   +-------------------------------------------------+
| Site                   INT                      |   | PK  journey           INT                       |
| EmitterCategory        INT DEFAULT 3            |   |     ident             VARCHAR                   |
| GBS                    INT                      |   |     model             VARCHAR                   |
| ModeA                  VARCHAR                  |   |     source            VARCHAR                   |
| PK  TimeRecPosition    DATETIME64(3, 'UTC')     |   |     location          INT                       |
| PK  AircraftAddress    VARCHAR                  |   | PK  timestamp         DateTime('UTC')           |
|     Latitude           DOUBLE                   |   |     latitude          DOUBLE                    |
|     Longitude          DOUBLE                   |   |     longitude         DOUBLE                    |
|     GeometricAltitude  DOUBLE                   |   |     altitude          INT                       |
|     FlightLevel        DOUBLE                   |   |     elevation         INT                       |
|     BarometricVertRate VARCHAR                  |   |     gps               INT                       |
|     GeoVertRateExceed  VARCHAR                  |   |     rssi              INT                       |
|     GeometricVertRate  VARCHAR                  |   |     home_lat          DOUBLE                    |
|     GroundSpeed        DOUBLE                   |   |     home_lon          DOUBLE                    |
|     TrackAngle         DOUBLE                   |   |     home_height       INT                       |
|     Callsign           VARCHAR                  |   |     speed             INT                       |
|     AircraftStopped    VARCHAR                  |   |     heading           INT                       |
|     GroundTrackValid   VARCHAR                  |   |     station_name      VARCHAR                   |
|     GroundHeadProvided VARCHAR                  |   |     station_latitude  DOUBLE                    |
|     MagneticNorth      VARCHAR                  |   |     station_longitude DOUBLE                    |
|     SurfaceGroundSpeed DOUBLE                   |   +-------------------------------------------------+
|     SurfaceGroundTrack DOUBLE                   |   Engine: MergeTree
+-------------------------------------------------+   Primary Key: (journey, timestamp)
Engine: MergeTree                                     Order By: (journey, timestamp)
Primary Key: (TimeRecPosition, AircraftAddress)

                         |                                            |                 |
                         v                                            v                 |
+-------------------------------------------------+   +-------------------------------+ |
|                    airplanes                    |   |            drones             | |
|                     (VIEW)                      |   |      (MATERIALIZED VIEW)      | |
+-------------------------------------------------+   +-------------------------------+ |
| Source: airplanes_raw                           |   | Source: drones_raw            | |
| Filter: prox_lat != 0 AND prox_lon != '0'       |   +-------------------------------+ |
+-------------------------------------------------+   | journey            INT        | |
| site                   INT                      |   | ident              VARCHAR    | |
| EmitterCategory        INT                      |   | model              VARCHAR    | |
| GBS                    BOOL (GBS == 1)          |   | source             VARCHAR    | |
| ModeA                  VARCHAR                  |   | timestamp          DateTime   | |
| time                   DATETIME64               |   | latitude           DOUBLE     | |
| prox_id                VARCHAR (AircraftAddress)|   | longitude          DOUBLE     | |
| prox_lat               DOUBLE                   |   | altitude           INT        | |
| prox_lon               DOUBLE                   |   | altitude_geo       Float64    | |
| prox_alt_m             DOUBLE                   |   | elevation          INT        | |
| flight_level           DOUBLE                   |   | home_lat           DOUBLE     | |
| baro_vert_rate         VARCHAR                  |   | home_lon           DOUBLE     | |
| geo_vert_exceeded      BOOL                     |   | home_height        INT        | |
| geo_vert_rate          VARCHAR                  |   | speed              INT        | |
| ground_speed           DOUBLE                   |   | heading            INT        | |
| TrackAngle             DOUBLE                   |   | station_name       VARCHAR    | |
| prox_callsign          VARCHAR (sanitized)      |   | station_latitude   DOUBLE     | |
| stopped                BOOL                     |   | station_longitude  DOUBLE     | |
| GroundTrackValid       BOOL                     |   | PK  time           UInt32     | |
| GroundHeadingProvided  BOOL                     |   | PK  journey        INT        | |
| MagneticNorth          BOOL                     |   | home_distance_2d   Float64    | |
| SurfaceGroundSpeed     DOUBLE                   |   | home_distance_3d   Float64    | |
| SurfaceGroundTrack     DOUBLE                   |   +-------------------------------+ |
+-------------------------------------------------+   Engine: ReplacingMergeTree        |
                                                      Primary Key: (time, journey)      |
                                                                                        |
                                                                                        |
                                +-------------------------------------------------------+
                                |
                                v
+---------------------------------------------------------------------------------------+
|                                      pbi_drones                                       |
|                                        (VIEW)                                         |
+---------------------------------------------------------------------------------------+
| Source: drones_raw dr LEFT OUTER JOIN pbi_deployments d                               |
| Join: dr.station_name = d.antenna_name AND dr.timestamp BETWEEN d.start_at AND d.end_at|
| Filter: dr.station_name != 'ASDSTATIONV1' AND sitename != ''                          |
+---------------------------------------------------------------------------------------+
| journey                  INT              | elevation_m              INT              |
| ident                    VARCHAR          | home_lat                 DOUBLE           |
| model                    VARCHAR          | home_lon                 DOUBLE           |
| installation_id          INT              | drone_reported_height_m  INT              |
| sitename                 VARCHAR          | speed_m_s                DOUBLE           |
| site_id                  INT              | heading                  INT              |
| date                     Date             | station_name             VARCHAR          |
| utc_time                 String           | station_latitude         DOUBLE           |
| local_date               Date             | station_longitude        DOUBLE           |
| local_time               String           | time                     UInt32           |
| drone_lat                DOUBLE           | home_distance_2d         Float64          |
| drone_lon                DOUBLE           | home_distance_3d         Float64          |
| drone_alt_m              INT              | antenna_distance_2d      Float64          |
| drone_alt_geo_m          Float64          | antenna_distance_3d      Float64          |
| drone_height_m           INT              |                                           |
+---------------------------------------------------------------------------------------+

===================================================================================================================
                                      ENCOUNTER & PROXIMITY TABLES & VIEWS
===================================================================================================================

+---------------------------------------------------------------------------------------+
|                                     airplane_prox                                     |
|                                        (TABLE)                                        |
+---------------------------------------------------------------------------------------+
| Generated by: process-data distances calculation pipeline                             |
| Engine: ReplacingMergeTree                                                            |
| Primary Key: (time, journey)                                                          |
+---------------------------------------------------------------------------------------+
| site_id            INT                    | prox_lat           FLOAT                  |
| sitename           VARCHAR                | prox_lon           FLOAT                  |
| en_id              VARCHAR                | prox_alt_m         FLOAT                  |
| PK  time           TIMESTAMP              | prox_mode_a        VARCHAR                |
| PK  journey        INT                    | prox_ecat          INT                    |
| drone_id           VARCHAR                | distance_slant_m   INT                    |
| model              VARCHAR                | distance_hor_m     INT                    |
| drone_lat          FLOAT                  | distance_vert_m    INT                    |
| drone_lon          FLOAT                  | distance_home_m    INT                    |
| drone_alt_m        FLOAT                  | station_name       VARCHAR                |
| drone_height_m     FLOAT                  |                                           |
| prox_callsign      VARCHAR                |                                           |
| prox_id            VARCHAR                |                                           |
+---------------------------------------------------------------------------------------+
                    |                                                   |
                    |                                                   |
                    v                                                   v
+---------------------------------------+   +-------------------------------------------+
|            airprox_summary            |   |               pbi_encounters              |
|                (VIEW)                 |   |            (MATERIALIZED VIEW)            |
+---------------------------------------+   +-------------------------------------------+
| Source: airplane_prox                 |   | Sources: airplane_prox ap,                |
| Group By: en_id, journey,             |   |          pbi_deployments d, sites s       |
|           drone_id, station_name      |   | Join: ap.site_id = s.id, s.name=d.sitename|
+---------------------------------------+   | Engine: ReplacingMergeTree                |
| en_id            VARCHAR              |   | Primary Key: (time, journey)              |
| journey          INT                  |   +-------------------------------------------+
| drone_id         VARCHAR              |   | en_id, installation_id, site_id, sitename |
| station_name     VARCHAR              |   | station_name, time, date, utc_time,       |
| distance_slant_m INT (MIN)            |   | local_date, local_time, journey, drone_id |
+---------------------------------------+   | model, drone_lat, drone_lon, drone_alt_m  |
                    |                       | drone_height_m, prox_callsign, prox_id    |
                    |                       | prox_lat, prox_lon, prox_alt_m,           |
                    |                       | prox_mode_a, prox_ecat, distance_slant_m  |
                    |                       | distance_hor_m, distance_vert_m,          |
                    |                       | distance_home_m                           |
                    |                       +-------------------------------------------+
                    |                                             |
                    +----------------------+----------------------+
                                           |
                                           v
+---------------------------------------------------------------------------------------+
|                                pbi_encounters_summary                                 |
|                                  (MATERIALIZED VIEW)                                  |
+---------------------------------------------------------------------------------------+
| Source: pbi_encounters p JOIN airprox_summary s                                       |
| Join: s.en_id = p.en_id AND s.journey = p.journey AND                                 |
|       s.drone_id = p.drone_id AND s.station_name = p.station_name                     |
| Filter: p.distance_slant_m = s.distance_slant_m                                       |
| Engine: ReplacingMergeTree                                                            |
| Primary Key: (en_id)                                                                  |
+---------------------------------------------------------------------------------------+
| Contains closest approach point for each encounter ID (en_id) with full PBI metadata  |
+---------------------------------------------------------------------------------------+

===================================================================================================================
                                            OPERATIONAL STATS
===================================================================================================================

+---------------------------------------------------------------------------------------+
|                                      daily_stats                                      |
|                                        (TABLE)                                        |
+---------------------------------------------------------------------------------------+
| Engine: ReplacingMergeTree                                                            |
| Primary Key: (day, site_name)                                                         |
| Comment: Records daily execution history and metrics for all sites                    |
+---------------------------------------------------------------------------------------+
| PK  day            DATE                   | status           INT NOT NULL             |
|     site_id        INT (FK -> sites.id)   | stats            VARCHAR                  |
| PK  site_name      VARCHAR                | comment          VARCHAR                  |
+---------------------------------------------------------------------------------------+
```

---

## 3. SQL User-Defined Functions (Macros)

The setup module defines user-defined SQL functions used throughout views and distance calculation queries:

```
+---------------------------------------------------------------------------------------+
|                              USER DEFINED FUNCTIONS (UDF)                             |
+--------------------+------------------------------------------------------------------+
| Function Name      | SQL Definition & Purpose                                         |
+--------------------+------------------------------------------------------------------+
| dist_2d            | geoDistance(dx, dy, px, py)                                      |
|                    | Calculates 2D geodesic distance in meters between two lat/lon.   |
+--------------------+------------------------------------------------------------------+
| dist_3d            | sqrt(pow(geoDistance(dx, dy, px, py), 2) + pow((dz - pz), 2))    |
|                    | Calculates 3D Euclidean distance in meters combining geodesic    |
|                    | surface distance with altitude difference.                       |
+--------------------+------------------------------------------------------------------+
| dist_3d_sq         | pow(geoDistance(dx, dy, px, py), 2) + pow(dz - pz, 2)            |
|                    | Squared 3D distance, avoiding expensive sqrt() for thresholds.   |
+--------------------+------------------------------------------------------------------+
| which_timezone     | SELECT timezone FROM deployments WHERE install_id = id           |
|                    | Resolves site timezone string for a given installation ID.       |
+--------------------+------------------------------------------------------------------+
```

---

## 4. Setup Command Mapping (`process-data setup`)

The following table summarizes the CLI flags to the corresponding database schema elements created:

| CLI Option | Function in `setup/` | Database Objects Created | Type |
|---|---|---|---|
| `--core --force` | `create_airplanes_raw_table`, `create_drones_raw_table` | `airplanes_raw`, `drones_raw` | Tables |
| `-M`, `--macros` | `add_macros` | `dist_2d`, `dist_3d`, `dist_3d_sq`, `which_timezone` | UDF / Functions |
| `--metadata` | `add_sites_table`, `add_antennas_table`, `add_installations_table` | `sites`, `antennas`, `installations` | Tables |
| `-P`, `--airplanes`| `add_airplanes_view` | `airplanes` | View |
| `-D`, `--drones` | `add_drones_view` | `drones` | Materialized View |
| `-E`, `--encounters`| `add_encounters_table` | `airplane_prox` | Table |
| `-R`, `--records` | `add_daily_stats_table` | `daily_stats` | Table |
| `-V`, `--views` | `create_work_views` | `deployments`, `pbi_deployments`, `pbi_drones`, `airprox_summary`, `pbi_encounters`, `pbi_encounters_summary` | Views / Mat Views |
| `-a`, `--all` | `setup_acute_environment` | Creates all macros, tables, and views | All |
