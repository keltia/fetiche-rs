# process-data

This is a CLI utility to load csv file containing ADS-B data and save them as Parquet files.

It is used to load and convert CSV files from the EMIT surveillance network into smaller and more usable [Parquet]
files.

This utility uses Clickhouse and the tables/SQL as explained in [SCHEMA.md](../docs/SCHEMA.md)

## USAGE

There are several ways to specify which instance and database of Clickhouse to use.

- environment variables

you can use `$CLICKHOUSE_CLIENT` to specify the Clickhouse instance and `$CLICKHOUSE_DB`, `$CLICKHOUSE_USER` and
`$CLICKHOUSE_PASSWORD` to specify database, user and password for the given instance.

- `process-data.hcl` this configuration file resides in the `drone-utils` main directory, like many others in Fetiche.

```hcl
version = 2

datalake = "/path/to/datalake"

url      = "http://SOME.HOST.NAME:8123"
database = "acute"
user     = "WHOEVER"
password = "HIDDEN"
```

<details>
<summary>`process-data --help`</summary>

```text
Implement different processing of data.

Usage: process-data [OPTIONS] <COMMAND>

Commands:
  acute       Display data about Acute sites, etc
  distances   Distance-related calculations
  export      Export results as CSV
  cleanup     Remove macros and other stuff
  setup       Prepare the database environment with some tables and macros
  completion  Generation completion stuff for shells
  version     List all package versions
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>      Alternate Configuration file
  -d, --database <DATABASE>  Database file to use
  -h, --help                 Print help
  ```

</details>

# Available tasks

## Proximity calculation

Using drone data from ASD and ADS-B data from EMIT, calculate various metrics:

For a given site, process data and calculate

- 3D distance between a drone and nearby airplanes
- 2D distance
- Number of encounters below 1nm, between 1 and 3 nm
- Save data about each encounter for both drone & airplane

```text
Distance-related calculations

Usage: process-data distances [OPTIONS] <COMMAND>

Commands:
  home    2D/3D drone to operator distance
  planes  drone to planes distance
  help    Print this message or the help of the given subcommand(s)

Options:
  -o, --output <OUTPUT>  Output file (default is stdout)
  -h, --help             Print help
```

### Data selection

- sites, antennas, etc.

```text
Display data about Acute sites, etc

Usage: process-data acute [OPTIONS] --database <DATABASE> <COMMAND>

Commands:
  antennas       Display all antennas
  installations  Fetch which antenna was on a site and when
  sites          Display all sites
  help           Print this message or the help of the given subcommand(s)

Options:
  -d, --database <DATABASE>  Database file to use
  -o, --output <OUTPUT>      Output file (default is stdout)
  -h, --help                 Print help
```

We export weekly statistics about drone traffic for each site we have an antenna.

```text
Export results as CSV

Usage: process-data export [OPTIONS] <COMMAND>

Commands:
  distances  Export the distance calculations
  drones     Export daily or weekly stats for drones
  help       Print this message or the help of the given subcommand(s)

Options:
  -d, --database <DATABASE>  Database file to use
  -h, --help                 Print help
```

## Trajectory categorisation

Using an ML system to classify the different kind of trajectory we can expect from a drone. Requires binding to python.

## Performance

## TODO

- Implement [Clickhouse] support **WIP**
- Implement a better way to manage migrations
- Move some of the code dealing with DB into the DB itself through UDF (User Defined Functions)

[datafusion]: https://crates.io/crates/arrow-datafusion

[Clickhouse]: https://clickhouse.com/
