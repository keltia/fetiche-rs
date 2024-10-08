# Various scripts for different systems

## `direnv`

The `/acute`  directory tree now use [direnv] to automatically add environment variables based on your location.  
The `env.sh` is linked as `.envrc` in `/acute/import` and it will be read every time your shell enter said
directories.  
Remember to use `direnv allow .` everywhere it is needed.

For `crontab(5)` usage, you should wrap the command like this:

```cronexp
10      0       *       *       *       cd /acute/import && direnv exec . /acute/bin/import-drones.py -D /acute .
```

## Environment variables

There are several environment variables used in several scripts, mostly to specify which [Clickhouse] instance is to be
used.
See `env.sh` above.

- `CLICKHOUSE_HOST`

FQDN to the DB server

- `CLICKHOUSE_URL`

URL for the HTTP client to [Clickhouse]. Format is `http://$CLICKHOUSE_HOST:8123` or `https://$CLICKHOUSE_HOST:8443`

- `CLICKHOUSE_DB`

Which DB to use

- `CLICKHOUSE_USER`
- `CLICKHOUSE_PASSWD`

## Shell (UNIX)

- `env.sh`

script to configure the current Clickhouse related environment variables (`CLICKHOUSE_*`) for all the scripts that
expect them to be defined when running. To be linked as `.envrc` where needed.

- `fetch-fa.sh`

Automate the retrieval of ADS-B data from [FlightAware] [Firehose] API through `acutectl`.

## Python (UNIX)

- `fetch-asd-drones.py`

Connect to the ASD API and fetch the latest drone data

```text
usage: fetch-asd-drones [-h] [--site SITE] [--datalake DATALAKE] [--keep]

Fetch the last dataset for drones from ASD API.

options:
  -h, --help            show this help message and exit
  --site SITE, -S SITE  Use this site.
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --keep, -K            Do not delete files after download.
```

- `fetch-all-drones.py`

Fetch all available drone data from the ASD site, either year by year or as a whole, create one file per day with the
drone data. Create a Hive-compatible tree in `<datalake>/drones/`. Site is the name of the account to be used to
connect.
See the configuration file `sources.hcl`.

```text
usage: fetch-all-drones [-h] [--site SITE] [--datalake DATALAKE] [--year YEAR]

Fetch all drone data from ASD in one go, creating the entire Hive-based directory tree.

options:
  -h, --help            show this help message and exit
  --site SITE, -S SITE  Use this site.
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --year YEAR, -Y YEAR  Fetch a specific year and not everything.
```

- `fetch-ftp-adsb.py`

Connect to the FTP site `ftps.eurocontrol.fr` and retrieve the latest CSV files for ADS-B data for our various stations.

```text
usage: fetch-ftp-adsb [-h] [--datalake DATALAKE] [--keep]

Fetch the last files from the incoming directory on ftps.

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --keep, -K            Do not delete files after download.
```

## Python (both Windows and UNIX)

- `import-adsb.py`

Import a file or a tree of files in parquet or csv format into a [Clickhouse] instance. This version is specific
to the `airplanes_raw` ADS-B table.

```text
usage: import-adsb [-h] [--datalake DATALAKE] [--dry-run] [--delete] [files ...]

Import ADS-B data into CH.

positional arguments:
  files                 List of files or directories.

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --dry-run, -n         Just show what would happen.
  --delete, -d          Delete final file.
```

- `import-drones.py`

Import a file or a tree of files in parquet or csv format into a [Clickhouse] instance. This version is specific
to the `drones_raw` table.

```text
usage: import-drones [-h] [--datalake DATALAKE] [--dry-run] [files ...]

Import drone data into CH.

positional arguments:
  files                 List of files or directories.

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --dry-run, -n         Do not actually move the file.
```

- `location-to-geojson.py`

Take the Lat/Lon coordinates from `locations.hcl` and generate a proper GeoJSON file for DuckDB.

- `fetch-opensky.py`

Python-only version of `opensky-history`, created to work around an incompatibility between the current "nightly" of
Rust and `inline-python`.

```text
usage: fetch-opensky [-h] [-o OUTPUT] [-B BEGIN] [-E END] [-P] [--today] [--yesterday] file location

Fetch OpenSky historical ADS-B data.

positional arguments:
  file                  HCL file with station locations.
  location              Location like BRU or LUX.

options:
  -h, --help            show this help message and exit
  -o OUTPUT, --output OUTPUT
                        Output file.
  -B BEGIN, --begin BEGIN
                        Start of time period.
  -E END, --end END     End of time period.
  -P, --parquet         Parquet output.
  --today               Only traffic for today.
  --yesterday           Only traffic for yesterday.
```

- `convert-csv.py`

This is the equivalent of `convert-ps1` but using Python and [bdt] instead of the soon-to-be obsolete `adsb-to-parquet`
because the former has more options and my pull request adding `-s` and `-z` has been merged.

```text
usage: convert-csv [-h] [--dry-run] [--delete] [files ...]

Uncompress and convert every csv file into parquet.

positional arguments:
  files          List of files or directories.

options:
  -h, --help     show this help message and exit
  --dry-run, -n  Do not actually move the file.
  --delete, -d   Remove csv after conversion.
```

- `dispatch-drops.py`

Take the parquet files coming from `convert-csv.py` and move them into the proper Hive subdirectory.

```text
usage: dispatch-drops [-h] [--datalake DATALAKE] [--drones] [--dry-run] [files ...]

Move each file in the right Hive directory for the given day.

positional arguments:
  files                 List of files or directories.

options:
  -h, --help            show this help message and exit
  --datalake DATALAKE, -D DATALAKE
                        Datalake is here.
  --drones              This is drone data.
  --dry-run, -n         Do not actually move the file.
```

## Obsolete utilities

- `convert-csv.ps1`

Automate the conversion of several CSV files into their Parquet equivalent on Windows.

## References

[bdt]: https://github.com/datafusion-contrib/bdt

[Clickhouse]: https://clickhouse.com/

[direnv]: https://direnv.net/

[Firehose]: https://www.flightaware.com/firehose/documentation/

[Flightaware]: https://flightaware.com/
