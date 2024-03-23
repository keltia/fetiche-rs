# Description of the various tables & queries and their implementation

We consider two databases:

- DuckDB: currently used as an embedded database so it is fast as there is no latency due to network
- Clickhouse: coming from Yandex & Cloudflare
- Databend: was the first candidate, but it is lacking in Geo functions

## DuckDB

### Macros

```sql
CREATE
OR REPLACE MACRO nm_to_deg(nm) AS
  nm * 1.852 / 111111.11;
CREATE
OR REPLACE MACRO deg_to_m(deg) AS
  deg * 111111.11;
CREATE
OR REPLACE MACRO m_to_deg(m) AS
  m / 111111.11;
CREATE
OR REPLACE MACRO dist_2d(px, py, dx, dy) AS
  CEIL(ST_Distance_Spheroid(ST_Point(py, px), ST_Point(dy, dx)));
CREATE
OR REPLACE MACRO dist_3d(px, py, pz, dx, dy, dz) AS
  CEIL(SQRT(POW(dist_2d(px, py, dx, dy), 2) + POW((pz - dz), 2)));
```

### Tables

```sql
-- Store data for the sites
--
CREATE TABLE sites
(
    id        INTEGER PRIMARY KEY,
    name      VARCHAR NOT NULL,
    code      VARCHAR NOT NULL,
    latitude  FLOAT   NOT NULL,
    longitude FLOAT   NOT NULL,
);
```

```sql
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
```

```sql
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

```sql
CREATE
OR REPLACE TABLE airplane_prox (
  site             VARCHAR,
  en_id            VARCHAR,
  time             TIMESTAMP,
  journey          INT,
  drone_id         VARCHAR,
  model            VARCHAR,
  drone_lon        FLOAT,
  drone_lat        FLOAT,
  drone_alt_m      FLOAT,
  drone_height_m   FLOAT,
  prox_callsign    VARCHAR,
  prox_id          VARCHAR,
  prox_lon         FLOAT,
  prox_lat         FLOAT,
  prox_alt_m       FLOAT,
  distance_slant_m INT,
  distance_hor_m   INT,
  distance_vert_m  INT,
  distance_home_m  INT,
)
```

## Clickhouse

### Database

```sql
CREATE
DATABASE acute COMMENT 'ACUTE Project data.';
```

### Functions

```sql
CREATE FUNCTION dist_2d AS (dx, dy, px, py) ->
  ceil(geoDistance(dx,dy,px,py));
```

```sql
CREATE FUNCTION dist_3d AS (dx, dy, dz, px, py, pz) ->
  ceil(sqrt(pow(dist_2d(dx,dy,px,py), 2) + pow((dz-pz), 2)));
```

### Tables

```sql
-- Store data for the sites
--
CREATE TABLE acute.sites
(
    id        INTEGER PRIMARY KEY,
    name      VARCHAR NOT NULL,
    code      VARCHAR NOT NULL,
    latitude  FLOAT   NOT NULL,
    longitude FLOAT   NOT NULL,
) ENGINE MergeTree;
```

```sql
-- Store one antenna
--
CREATE TABLE acute.antennas
(
    id          INTEGER PRIMARY KEY,
    type        VARCHAR,
    name        VARCHAR NOT NULL,
    owned       BOOLEAN,
    description VARCHAR,
) ENGINE MergeTree;
```

```sql
-- Store one installation of one antenna on one site 1:1
--
CREATE TABLE acute.installations
(
    id         INTEGER PRIMARY KEY,
    site_id    INTEGER,
    antenna_id INTEGER,
    start_at   TIMESTAMP,
    end_at     TIMESTAMP,
    comment    VARCHAR,
    FOREIGN KEY (site_id) REFERENCES sites (id),
    FOREIGN KEY (antenna_id) REFERENCES antennas (id),
) ENGINE MergeTree;
```

```sql
CREATE
OR REPLACE TABLE acute.airplane_prox (
  site             VARCHAR,
  en_id            VARCHAR,
  time             TIMESTAMP,
  journey          INT,
  drone_id         VARCHAR,
  model            VARCHAR,
  drone_lon        FLOAT,
  drone_lat        FLOAT,
  drone_alt_m      FLOAT,
  drone_height_m   FLOAT,
  prox_callsign    VARCHAR,
  prox_id          VARCHAR,
  prox_lon         FLOAT,
  prox_lat         FLOAT,
  prox_alt_m       FLOAT,
  distance_slant_m INT,
  distance_hor_m   INT,
  distance_vert_m  INT,
  distance_home_m  INT,
) ENGINE = MergeTree PRIMARY KEY (time, journey);
```



