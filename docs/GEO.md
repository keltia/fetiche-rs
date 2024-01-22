## DuckDB and Geospatial

### Tables

Manually managed tables:

```sql
-- Store the site's data incl Aeroscope ID and time interval
--
CREATE TABLE sites
(
    id        INTEGER PRIMARY KEY,
    name      VARCHAR NOT NULL,
    code      VARCHAR NOT NULL,
    latitude  FLOAT   NOT NULL,
    longitude FLOAT   NOT NULL,
);

-- Store one antenna
--
CREATE TABLE antennas
(
    id          INTEGER PRIMARY KEY,
    type        VARCHAR,
    name        VARCHAR NOT NULL,
    owned       BOOLEAN,
    description VARCHAR,
);

-- Store one installation of one antenna on one site 1:1
--
CREATE TABLE installations
(
    id         INTEGER PRIMARY KEY,
    site_id    INTEGER,
    antenna_id INTEGER,
    start_at   TIMESTAMP_NS NOT NULL,
    end_at     TIMESTAMP_NS NOT NULL,
    FOREIGN KEY (site_id) REFERENCES sites (id),
    FOREIGN KEY (antenna_id) REFERENCES antennas (id),
);
```

### import data from parquet

```text
create table luxemburg as select * from read_parquet(['Lux*.parquet']);
```

- select what we need from ADS-B

```text
D select "073.TimeRecPosition","131.Longitude","131.Latitude" from luxemburg;
┌─────────────────────────┬───────────────┬───────────────┐
│   073.TimeRecPosition   │ 131.Longitude │ 131.Latitude  │
│         varchar         │    double     │    double     │
├─────────────────────────┼───────────────┼───────────────┤
│ 2023-11-21 00:00:00.080 │   6.653378047 │ 49.6207579784 │
│ 2023-11-21 00:00:00.210 │  7.9039586708 │  49.280044511 │
│ 2023-11-21 00:00:00.120 │  6.5631865896 │ 50.8232192695 │
│ 2023-11-21 00:00:00.010 │  8.7852172181 │ 47.9100951925 │
│ 2023-11-20 23:59:59.410 │  9.8867092654 │  49.213760253 │
│ 2023-11-21 00:00:00.030 │  6.4490450174 │ 47.6533843204 │
```

- Transform into proper timestamp

```text
D alter table luxemburg alter "073.TimeRecPosition" set data type timestamp_ns using strptime("073.TimeRecPosition", '%Y-%m-%d %H:%M:%S.%g');
D select "073.TimeRecPosition","131.Longitude","131.Latitude","140.GeometricAltitude" from luxemburg;
┌────────────────────────┬───────────────┬───────────────┬───────────────────────┐
│  073.TimeRecPosition   │ 131.Longitude │ 131.Latitude  │ 140.GeometricAltitude │
│      timestamp_ns      │    double     │    double     │        double         │
├────────────────────────┼───────────────┼───────────────┼───────────────────────┤
│ 2023-11-21 00:00:00.08 │   6.653378047 │ 49.6207579784 │               39550.0 │
│ 2023-11-21 00:00:00.21 │  7.9039586708 │  49.280044511 │               36575.0 │
│ 2023-11-21 00:00:00.12 │  6.5631865896 │ 50.8232192695 │               23850.0 │
│ 2023-11-21 00:00:00.01 │  8.7852172181 │ 47.9100951925 │               33700.0 │
│ 2023-11-20 23:59:59.41 │  9.8867092654 │  49.213760253 │               31600.0 │
│ 2023-11-21 00:00:00.03 │  6.4490450174 │ 47.6533843204 │               33575.0 │
│ 2023-11-21 00:00:00.2  │  5.8388107829 │ 47.7518889494 │               36650.0 │
│ 2023-11-20 23:59:59.13 │  8.4158886038 │ 48.8818979077 │               33575.0 │
│ 2023-11-20 23:59:59.3  │  3.8404083252 │  50.793658644 │               36525.0 │
│ 2023-11-21 00:00:00.23 │  6.2144757807 │   50.96118154 │               23025.0 │
```

- Add columns for 2D and 3D points

```text

```

## Crates to look at

- geo
- geozero

## Sites

- [PRQL](https://prql-lang.org/)





