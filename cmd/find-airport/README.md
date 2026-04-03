# convert-from-json

A simple CLI interface to the airport data hosted by [OurAirports](https://ourairports.com/), part of the **fetiche**
ecosystem. This utility fetch the csv file, and converts it into parquet. [Polars](https://pola.rs) is used to
access the data.

## Features

- Can load, refresh and clean up the data file from the [OurAirports](https://ourairports.com/data/) site.
- Converts the various CSV files to Parquet format.
- Can query all the data files about specific airports, list of airports, etc.
- All queries can be converted to CSV, JSON or NDJSON, the default is plain text with tables, as shown below.

## Usage

```shell
Fetch OurAirports data into Parquet, and search it.

Usage: find-airport [OPTIONS] <COMMAND>

Commands:
  clean  Remove the current parquet files from the datalake
  fetch  Fetch or refresh the latest parquet files from the datalake
  find   Find airports by IATA code, ICAO code, name, or country
  show   Show the current parquet files in the datalake
  help   Print this message or the help of the given subcommand(s)

Options:
  -D, --datalake <DATALAKE>  Directory holding the parquet files.
  -L, --use-tree             Enable logging in a hierarchical manner (aka tree)
  -F, --use-file <USE_FILE>  This parameter enables logging to a file in that location
  -q, --quiet                We do not want anything more than the data
  -V, --version              Display version
  -n, --dry-run              Dry run
  -h, --help                 Print help```

Most of the commands are self-explanatory, but `find` deserve more, because it has many options.

```shell
Find airports by IATA code, ICAO code, name, or country

Usage: find-airport find [OPTIONS] <TEXT>

Arguments:
  <TEXT>  Search text

Options:
  -C, --country    Display by country code
  -I, --icao       Find by ICAO code
  -A, --iata       Airport IATA code
  -N, --name       Find by searching in the name
  -F, --fmt <FMT>  Output as CSV/JSON/NDJSON/Plain [default: plain]
  -h, --help       Print help
```

## Configuration file

It uses an HCL (Hashicorp Configuration Language) file to specify the location of the data files, where to fetch them
from and which ones to get. The file resides in `$HOME/.config/drone-utils/airports.hcl` ($env:LOCALAPPDATA/drone-utils`
on Windows, as usual).

```hcl
version = 1

// Basedir for everything -- will load files into "{datalake}/files"
datalake = "/acute/datalake"

// Source site
base_url = "https://davidmegginson.github.io/ourairports-data/"

// We do not need everything, these are csv, will be converted to parquet
sources = ["airports", "airport-frequencies", "navaids", "runways"]
```

> NOTE: only `airports.csv` is currently used, Others are fetched and will be used later.
>

## Examples

### Data Management

Bootstrapping the data:

```text
❯ find-airport fetch

find-airport/0.2.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
Fetch OurAirports data into Parquet, and search it.

Repository: /acute/datalake/files

Files:
┌─────────┬─────────────────────────────┬──────────────────────────────┬─────────┬───────┐
│ status  │ name                        │                        mtime │    size │  rows │
├─────────┼─────────────────────────────┼──────────────────────────────┼─────────┼───────┤
│ Present │ airports.parquet            │  2026-03-15T20:29:43.582095Z │ 3370633 │ 84811 │
│ Present │ airport-frequencies.parquet │ 2026-03-15T20:29:43.1694483Z │  318539 │ 30216 │
│ Present │ navaids.parquet             │ 2026-03-15T20:29:43.2976231Z │  402012 │ 11010 │
│ Present │ runways.parquet             │ 2026-03-15T20:29:43.2541558Z │  922333 │ 47683 │
└─────────┴─────────────────────────────┴──────────────────────────────┴─────────┴───────┘
```

Showing the current data:

```text
❯ find-airport show

find-airport/0.2.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
Fetch OurAirports data into Parquet, and search it.

Repository: /acute/datalake/files

Files:
┌─────────┬─────────────────────────────┬──────────────────────────────┬─────────┬───────┐
│ status  │ name                        │                        mtime │    size │  rows │
├─────────┼─────────────────────────────┼──────────────────────────────┼─────────┼───────┤
│ Present │ airports.parquet            │ 2026-03-14T09:45:29.7144974Z │ 3370328 │ 84807 │
│ Present │ airport-frequencies.parquet │ 2026-03-14T09:45:29.4014898Z │  318539 │ 30216 │
│ Present │ navaids.parquet             │ 2026-03-14T09:45:29.5114894Z │  402012 │ 11010 │
│ Present │ runways.parquet             │ 2026-03-14T09:45:29.4672031Z │  922278 │ 47681 │
└─────────┴─────────────────────────────┴──────────────────────────────┴─────────┴───────┘
```

### Queries

By IATA code:

```text
❯ find-airport.exe find -A CDG

find-airport/0.2.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
Fetch OurAirports data into Parquet, and search it.

Repository: /acute/datalake/files
Looking for airport: CDG

Found by IATA/ICAO/Name/Country:

┌───────┬─────────────────────────────────────────┬───────────┬───────────┬─────────────┬───────────┬──────────────┬────────┬───────────┐
│ ident │ name                                    │  Latitude │ Longitude │ elevation_m │ iata_code │ timezone     │ offset │ pluscode  │
├───────┼─────────────────────────────────────────┼───────────┼───────────┼─────────────┼───────────┼──────────────┼────────┼───────────┤
│ LFPG  │ Charles de Gaulle International Airport │ 49.008960 │  2.554117 │         119 │    CDG    │ Europe/Paris │   1    │ 8FX42H53+ │
└───────┴─────────────────────────────────────────┴───────────┴───────────┴─────────────┴───────────┴──────────────┴────────┴───────────┘
```

By Country:

```text
❯ find-airport.exe find -C IS

find-airport/0.2.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
Fetch OurAirports data into Parquet, and search it.

Repository: /acute/datalake/files
Looking for airport: IS

Found by IATA/ICAO/Name/Country:

┌───────┬────────────────────────────────┬───────────┬────────────┬─────────────┬───────────┬────────────────────┬────────┬───────────┐
│ ident │ name                           │  Latitude │  Longitude │ elevation_m │ iata_code │ timezone           │ offset │ pluscode  │
├───────┼────────────────────────────────┼───────────┼────────────┼─────────────┼───────────┼────────────────────┼────────┼───────────┤
│ BIAR  │ Akureyri International Airport │ 65.656573 │ -18.072018 │           1 │    AEY    │ Atlantic/Reykjavik │   0    │ 9CQ3MW4H+ │
 [...]
│ BIVM  │ Vestmannaeyjar Airport         │ 63.424301 │ -20.278900 │          99 │    VEY    │ Atlantic/Reykjavik │   0    │ 99MXCPFC+ │
│ BIVO  │ Vopnafjörður Airport           │ 65.720596 │ -14.850600 │           4 │    VPN    │ Atlantic/Reykjavik │   0    │ 9CQ7P4CX+ │
└───────┴────────────────────────────────┴───────────┴────────────┴─────────────┴───────────┴────────────────────┴────────┴───────────┘
```

