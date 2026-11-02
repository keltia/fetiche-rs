# ClickHouse Schema Optimization & Improvement Guide

This document outlines structural, architectural, and performance recommendations for the **ACUTE / Fetiche-CH** ClickHouse database schema defined in `process-data/src/cmds/setup/`. 

These recommendations maximize query throughput, reduce disk and memory footprints, ensure deduplication determinism, and simplify long-term operational maintenance.

---

## Table of Contents

1. [Table Partitioning & Data Lifecycle (TTL)](#1-table-partitioning--data-lifecycle-ttl)
2. [Column Types, Encodings & Compression Codecs](#2-column-types-encodings--compression-codecs)
3. [Materialized View Architecture & Join Optimizations](#3-materialized-view-architecture--join-optimizations)
4. [Primary / Sorting Keys & ReplacingMergeTree Versioning](#4-primary--sorting-keys--replacingmergetree-versioning)
5. [Native Spatial & Timestamp Precision Alignment](#5-native-spatial--timestamp-precision-alignment)
6. [Naming Conventions & SQL Cleanliness](#6-naming-conventions--sql-cleanliness)
7. [Zero-Downtime Migration Strategy](#7-zero-downtime-migration-strategy)
8. [Summary Checklist](#8-summary-checklist)

---

## 1. Table Partitioning & Data Lifecycle (TTL)

### Add `PARTITION BY` Clauses to High-Volume Tables
Currently, high-volume ingestion tables (`airplanes_raw`, `drones_raw`, `airplane_prox`) do not specify a partition key. Without partitioning:
- Data parts span arbitrary ranges across the entire history.
- Time-filtered queries must evaluate sparse index marks across all historical data rather than skipping irrelevant parts entirely.
- Dropping, detaching, or backing up historical time ranges requires heavy table-wide mutations instead of instantaneous partition drops (`ALTER TABLE ... DROP PARTITION`).

**Recommendation**: Partition by month using `toYYYYMM(...)`:
```sql
-- airplanes_raw
PARTITION BY toYYYYMM(TimeRecPosition)

-- drones_raw
PARTITION BY toYYYYMM(timestamp)

-- airplane_prox
PARTITION BY toYYYYMM(time)
```

> **Granularity Guidance**: Monthly partitioning (`toYYYYMM`) is optimal for datasets ingesting under ~10–20 million rows per day. Avoid daily partitioning (`toYYYYMMDD`) unless daily volume exceeds tens of millions of rows, as excessive partitions create thousands of small parts and increase background merge overhead.

### Implement Data Retention Policies (TTL)
Raw telemetry streams (`airplanes_raw`, `drones_raw`) consume substantial storage over time relative to calculated encounter summaries (`airplane_prox`, `pbi_encounters_summary`).

**Recommendation**: Add table-level TTL declarations for automated cleanup or cold storage tiering:
```sql
-- Automatically prune raw drone telemetry older than 90 days
ALTER TABLE drones_raw MODIFY TTL timestamp + INTERVAL 90 DAY DELETE;

-- Automatically prune raw ADS-B telemetry older than 90 days
ALTER TABLE airplanes_raw MODIFY TTL TimeRecPosition + INTERVAL 90 DAY DELETE;
```

---

## 2. Column Types, Encodings & Compression Codecs

### Eliminate Inefficient String Flags
In `airplanes_raw`, several boolean status indicators are declared as `VARCHAR` (e.g., `'0'` or `'1'`), leading to string parsing in downstream views (`AircraftStopped == '1'`):
- `AircraftStopped`
- `GroundTrackValid`
- `GroundHeadingProvided`
- `MagneticNorth`
- `GeoVertRateExceeded`

**Recommendation**: Store flags as native `Bool` or `UInt8`. This reduces storage by 87% on these columns and accelerates boolean evaluations into SIMD operations.

### Apply `LowCardinality(String)` to Dictionary Columns
ClickHouse `LowCardinality` dictionary-encodes distinct string values, saving memory during aggregations/groupings and shrinking on-disk index structures:

**Candidates for `LowCardinality(String)`**:
- `station_name`, `model`, `source`, `ModeA` (in `drones_raw`, `airplanes_raw`, `airplane_prox`)
- `type`, `antenna_name`, `site_name`, `timezone` (in `antennas`, `sites`, `deployments`)

```sql
station_name LowCardinality(String),
model        LowCardinality(String),
source       LowCardinality(String)
```

### Apply Column Compression Codecs
Applying specialized column codecs optimizes compression ratios (often 2x–5x higher than default LZ4) and lowers I/O bottlenecks during scans:

| Data Type / Column | Recommended Codec | Rationale |
| :--- | :--- | :--- |
| **Timestamps / Dates** (`timestamp`, `time`, `TimeRecPosition`) | `CODEC(DoubleDelta, ZSTD)` | Monotonically increasing time points compress exceptionally well with double-delta encoding. |
| **Coordinates** (`latitude`, `longitude`, `prox_lat`, `prox_lon`) | `CODEC(Gorilla, ZSTD)` | Floating-point time-series algorithms (Gorilla) yield maximum compression on continuous coordinate variations. |
| **Monotonic IDs / Heights / Distances** (`journey`, `altitude`, `distance_*`) | `CODEC(T64, ZSTD)` or `CODEC(Delta, ZSTD)` | Integer truncation and delta codecs minimize byte width for bounded numeric values. |
| **General Text / Identifiers** (`ident`, `prox_callsign`, `prox_id`) | `CODEC(ZSTD(1))` | Higher compression ratio than standard LZ4 with negligible CPU decompression overhead. |

---

## 3. Materialized View Architecture & Join Optimizations

### Decouple Materialized Views from Internal Storage
Currently, materialized views such as `drones` and `pbi_encounters` use inline definitions (`CREATE MATERIALIZED VIEW ... ENGINE = ReplacingMergeTree AS SELECT ...`).

**Recommendation**: Adopt the explicit target table pattern:
1. Create a dedicated target storage table (`CREATE TABLE target_drones (...) ENGINE = ...`).
2. Create the materialized view referencing the target table (`CREATE MATERIALIZED VIEW mv_drones TO target_drones AS SELECT ...`).

**Benefits**:
- Target tables can be altered, truncated, partitioned, or indexed independently without dropping the materialized view.
- Multiple ingestion pipelines or backfills can write directly to the target table without triggering duplicate view triggers.

### Remove Deprecated `POPULATE` Keyword
Using `POPULATE` during view creation blocks table creation and misses inserts that occur concurrently during the backfill process.

**Recommendation**: Create views without `POPULATE`, then backfill data explicitly via `INSERT INTO target_table SELECT ...`.

### Replace Range Joins with ClickHouse Dictionaries (`RANGE_HASHED`)
In `pbi_drones` and `pbi_encounters`, queries perform non-equality range joins against deployments:
```sql
dr.timestamp BETWEEN d.start_at AND d.end_at
```
In ClickHouse, range joins on large event streams are computationally expensive and memory-intensive.

**Recommendation**: Configure `installations` / `sites` as a `RANGE_HASHED` ClickHouse Dictionary (`dictGet(...)`). This converts heavy range joins into fast in-memory dictionary lookups.

### Clean Up Implicit Joins in `pbi_encounters`
In `views.rs`, `pbi_encounters` combines comma-separated tables with a `LEFT OUTER JOIN`:
```sql
FROM {workdb}.airplane_prox AS ap, {workdb}.pbi_deployments AS d
LEFT OUTER JOIN sites AS s ON ap.site_id = s.id
WHERE s.name = d.sitename
```
**Recommendation**: Refactor into explicit `INNER JOIN` clauses to eliminate intermediate Cartesian product ambiguity and improve query planner execution paths:
```sql
FROM {workdb}.airplane_prox AS ap
INNER JOIN {workdb}.sites AS s ON ap.site_id = s.id
INNER JOIN {workdb}.pbi_deployments AS d ON s.name = d.sitename
```

---

## 4. Primary / Sorting Keys & ReplacingMergeTree Versioning

### Add Version Columns to `ReplacingMergeTree` Tables
Tables using `ENGINE = ReplacingMergeTree` (`airplane_prox`, `drones`, `pbi_encounters`) currently do not specify a version column:
```sql
ENGINE = ReplacingMergeTree PRIMARY KEY (time, journey)
```
Without a version column, deduplication during background merges retains arbitrary rows when duplicate keys are encountered.

**Recommendation**: Add a dedicated version or timestamp column (`updated_at DateTime DEFAULT now()`) to the engine signature:
```sql
ENGINE = ReplacingMergeTree(updated_at)
ORDER BY (time, journey)
```

### Separate `PRIMARY KEY` and `ORDER BY`
In ClickHouse `MergeTree`, `ORDER BY` defines the physical on-disk sort order (which determines data compression and spatial locality), while `PRIMARY KEY` defines the sparse index in memory.
- If `PRIMARY KEY` is omitted, it defaults to `ORDER BY`.
- For tables with many sort columns, defining a shorter prefix for `PRIMARY KEY` reduces memory consumption of the primary index marks while maintaining efficient physical ordering.

---

## 5. Native Spatial & Timestamp Precision Alignment

### Utilize Native Vectorized Geo Functions
Custom UDF macros (`dist_2d`, `dist_3d`) use manual trigonometric calculations. ClickHouse provides native, SIMD-vectorized geographic functions:
- `geoDistance(lon1, lat1, lon2, lat2)`
- `greatCircleDistance(lon1, lat1, lon2, lat2)`

**Recommendation**: Update macro calculations and distance queries to leverage native built-in geo functions for significant execution speedups.

### Standardize Timestamp Precision Across Datasets
- `airplanes_raw` uses `DATETIME64(3, 'UTC')` (millisecond precision).
- `drones_raw` and `airplane_prox` use `DateTime('UTC')` (second precision).

**Recommendation**: Standardize telemetry and proximity timestamps to `DateTime64(3, 'UTC')`. High-speed drone trajectories and ADS-B proximity calculations benefit from millisecond precision to accurately order events occurring within the same second.

---

## 6. Naming Conventions & SQL Cleanliness

### Consistent Casing Across Tables
- `airplanes_raw` uses `PascalCase` (`TimeRecPosition`, `AircraftAddress`, `Latitude`, `Site`).
- `drones_raw`, `sites`, `antennas`, and `airplane_prox` use `snake_case` (`timestamp`, `station_name`, `site_id`).

**Recommendation**: Standardize all column names across all tables to `snake_case` to simplify cross-table joins, view definitions, and Rust struct mappings (`serde`, `clickhouse-rs`).

### Syntax Hygiene
- In `tables.rs` (`daily_stats`), remove trailing commas before closing parentheses (`comment VARCHAR,`) to ensure strict SQL parser compatibility across ClickHouse releases.

---

## 7. Zero-Downtime Migration Strategy

In ClickHouse, `PARTITION BY` cannot be added to an existing table via `ALTER TABLE`. To apply partitioning or fundamental column/engine updates without downtime, use the following atomic swap workflow:

```
+--------------------------+       +------------------------------+
|   airplanes_raw (Old)    |       |   airplanes_raw_new (New)    |
|   - Unpartitioned        |       |   - PARTITION BY toYYYYMM()  |
|   - Active Ingestion     |       |   - Optimized Codecs         |
+------------+-------------+       +--------------+---------------+
             |                                    |
             | 1. Bulk/Chunked Backfill           |
             +----------------------------------->|
             |                                    |
             | 2. Atomic EXCHANGE TABLES          |
             |<==================================>|
             |                                    |
+------------v-------------+       +--------------v---------------+
|   airplanes_raw (Active) |       |   airplanes_raw_new (Old)    |
|   - Partitioned & Fast   |       |   - Unpartitioned Data       |
+--------------------------+       +--------------+---------------+
                                                  |
                                                  | 3. Clean up
                                                  v
                                           +--------------+
                                           |  DROP TABLE  |
                                           +--------------+
```

### Migration Execution Steps

#### Step 1: Create the Replacement Table
```sql
CREATE TABLE acute.airplanes_raw_new
(
    site                     LowCardinality(String),
    emitter_category         UInt8 DEFAULT 3,
    gbs                      UInt8,
    mode_a                   LowCardinality(String),
    time_rec_position        DateTime64(3, 'UTC') CODEC(DoubleDelta, ZSTD),
    aircraft_address         String CODEC(ZSTD(1)),
    latitude                 Float64 CODEC(Gorilla, ZSTD),
    longitude                Float64 CODEC(Gorilla, ZSTD),
    geometric_altitude       Float64 CODEC(T64, ZSTD),
    flight_level             Float64 CODEC(T64, ZSTD),
    barometric_vert_rate     Float64,
    geo_vert_rate_exceeded   Bool,
    geometric_vert_rate      Float64,
    ground_speed             Float64 CODEC(T64, ZSTD),
    track_angle              Float64 CODEC(T64, ZSTD),
    callsign                 String CODEC(ZSTD(1)),
    aircraft_stopped         Bool,
    ground_track_valid       Bool,
    ground_heading_provided  Bool,
    magnetic_north           Bool,
    surface_ground_speed     Float64,
    surface_ground_track     Float64
)
ENGINE = MergeTree
PARTITION BY toYYYYMM(time_rec_position)
PRIMARY KEY (time_rec_position, aircraft_address)
ORDER BY (time_rec_position, aircraft_address);
```

#### Step 2: Backfill Data in Time Slices
For large datasets, backfill in monthly time slices to prevent memory exhaustion and reduce CPU spikes:
```sql
INSERT INTO acute.airplanes_raw_new
SELECT * FROM acute.airplanes_raw
WHERE TimeRecPosition >= '2026-01-01 00:00:00' AND TimeRecPosition < '2026-02-01 00:00:00';
```

#### Step 3: Atomic Table Swap
Instantly swap the active table reference with zero consumer downtime:
```sql
EXCHANGE TABLES acute.airplanes_raw AND acute.airplanes_raw_new;
```

#### Step 4: Drop Old Table
Drop the legacy table once data integrity and downstream queries are verified:
```sql
DROP TABLE acute.airplanes_raw_new;
```

---

## 8. Summary Checklist

- [ ] Add `PARTITION BY toYYYYMM(...)` to `airplanes_raw`, `drones_raw`, and `airplane_prox`.
- [ ] Set TTL data retention policies on high-volume raw telemetry.
- [ ] Convert string flags (`'0'`/`'1'`) to native `Bool` or `UInt8`.
- [ ] Convert low-cardinality fields (`station_name`, `model`, `source`, `timezone`) to `LowCardinality(String)`.
- [ ] Add `DoubleDelta`, `Gorilla`, and `T64` codecs to timestamps, coordinates, and metrics.
- [ ] Decouple Materialized Views (`drones`, `pbi_encounters`) into dedicated Target Table + `TO target_table` MV definitions.
- [ ] Remove `POPULATE` keyword from all view creation scripts.
- [ ] Add explicit version column (`updated_at DateTime`) to `ReplacingMergeTree` tables.
- [ ] Convert deployment range joins to `RANGE_HASHED` dictionaries.
- [ ] Standardize timestamp precision to `DateTime64(3, 'UTC')` across all tables.
- [ ] Standardize column naming to `snake_case`.
