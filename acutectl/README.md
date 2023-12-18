# acutectl

This is the main CLI interface to the Fetiche framework and engine.

## USAGE

<details>
<summary>`acutectl help`</summary>

```text
$ acutectl
CLI utility to fetch data.

Usage: acutectl [OPTIONS] <COMMAND>

Commands:
  completion  Generate Completion stuff
  fetch       Fetch data from specified site
  import      Import into InfluxDB (WIP)
  list        List information about formats and sources
  stream      Stream from a source
  version     List all package versions
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  configuration file
  -D, --debug            debug mode
  -o, --output <OUTPUT>  Output file
  -v, --verbose...       Verbose mode
  -h, --help             Print help
```

</details>

More detailed information in the specific [Aacutectl README.md](acutectl/README.md).

As seen, there are different sub-commands. You can use `acutectl help <sub-command>`  to get description of the
different parameters.

The `completion` keyword can be used to generate completion scripts for various shells including `zsh` on UNIX
and `powershell` on Windows.

### Configuration

Credentials are stored into the `acutectl` configuration file, located in the same directory but named, as one
can expect, `config.hcl`.

<details>
<summary>`config.hcl`</summary>

```hcl
version = 1

site "local" {
  username = "aeroscope"
  password = "NOPE"
  token    = "/login"
}

site "big.site.aero" {
  username = "SOMEONE"
  password = "HIDDEN"
  token    = "/auth"
}

site "opensky" {
  login    = "someone"
  password = "SECRET"
}

site "safesky" {
  api_key = "FOOBAR"
}

```

</details>

If you are just giving the utility a file, you must specify the input format with the `-F/--format` option.

### Formats

To displayed currently supported formats, use `acutectl list formats`:

<details>
<summary>acutectl list formats</summary>

```text
acutectl/0.13.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
CLI utility to fetch data.

List all formats:
┌─────────────┬───────┬───────────────────────────────────────────────────────────────────────────────────────────────┐
│ Name        │ Type  │ Description                                                                                   │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ aeroscope   │ drone │ Data extracted from the DJI Aeroscope antenna.                                                │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ asd         │ drone │ Data gathered & consolidated by ASD.                                                          │
│             │       │ Source: ASD -- URL: https://airspacedrone.com/                                                │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ avionix     │ adsb  │ Flattened ASTERIX cat21-like for Avionix stations.                                            │
│             │       │ Source: Avionix -- URL: http://www.avionix.pl                                                 │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ cat129      │ drone │ Flattened ASTERIX Cat129 data for Drone data.                                                 │
│             │       │ Source: ECTL -- URL: https://www.eurocontrol.int/asterix/                                     │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ cat21       │ adsb  │ Flattened ASTERIX Cat21 data for ADS-B.                                                       │
│             │       │ Source: ECTL -- URL: https://www.eurocontrol.int/asterix/                                     │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ flightaware │ adsb  │ ADS-B data by Flightaware.                                                                    │
│             │       │ Source: Flightaware -- URL: https://flightaware.com/commercial/firehose/documentation/summary │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ impala      │ adsb  │ Flattened StateVector extracted from Opensky Impala DB.                                       │
│             │       │ Source: Opensky -- URL: https://opensky-network.org/data/impala                               │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ opensky     │ adsb  │ Data coming from the Opensky site, mostly ADS-B.                                              │
│             │       │ Source: Opensky -- URL: https://opensky-network.org/                                          │
├─────────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────┤
│ safesky     │ adsb  │ Data coming from the Safesky site, mostly ADS-B.                                              │
│             │       │ Source: Safesky -- URL: https://www.safesky.app/                                              │
└─────────────┴───────┴───────────────────────────────────────────────────────────────────────────────────────────────┘
```

The reason for the different categories is to give the engine a hint on how to process the data.
</details>

There is also the concept of output or container formats. When saving data fetched from a site or during a conversion,
we can dump whatever is sent (JSON, CSV, etc.) or convert it into a container format like [Parquet].

You can get the list of supported container formats with `list containers`:
<details>
<summary>acutectl list containers</summary>

```text
acutectl/0.16.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
CLI utility to fetch data.

List all formats:
┌─────────┬───────┬───────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Name    │ Type  │ Description                                                                                           │
├─────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ ACSV    │ write │ Annotated CSV, created by InfluxDB.                                                                   │
│         │       │ Source: InfluxData -- URL: https://docs.influxdata.com/influxdb/cloud/reference/syntax/annotated-csv/ │
├─────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Avro    │ write │ Row-oriented tabular format from Apache Arrow.                                                        │
│         │       │ Source: Apache -- URL: https://avro.apache.org/                                                       │
├─────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ CSV     │ write │ Comma Separated Values aka your friend CSV.                                                           │
│         │       │ Source: IBM -- URL: https://en.wikipedia.org/wiki/CSV                                                 │
├─────────┼───────┼───────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Parquet │ write │ Apache Parquet export for drone/ADS-B data.                                                           │
│         │       │ Source: Apache -- URL: https://parquet.apache.org/docs/file-format/                                   │
└─────────┴───────┴───────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

</details>

### Sources

You can get the list of supported sources by using the `acutectl list sources` command.

<details>
<summary>`list sources`</summary>

```text
acutectl/0.11.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
CLI utility to fetch data.

Listing all sources:
╭─────────┬───────┬───────────┬───────────────────────────────────┬─────────┬──────────────╮
│ Name    │ Type  │ Format    │ URL                               │ Auth    │ Ops          │
├─────────┼───────┼───────────┼───────────────────────────────────┼─────────┼──────────────┤
│ eih     │ drone │ aeroscope │ http://127.0.0.1:2400             │ token   │ fetch        │
│ lux     │ drone │ asd       │ https://eur.airspacedrone.com/api │ token   │ fetch        │
│ lux-me  │ drone │ asd       │ https://eur.airspacedrone.com/api │ token   │ fetch        │
│ opensky │ adsb  │ opensky   │ https://opensky-network.org/api   │ login   │ fetch,stream │
│ safesky │ adsb  │ safesky   │ https://public-api.safesky.app    │ API key │ fetch        │
╰─────────┴───────┴───────────┴───────────────────────────────────┴─────────┴──────────────╯
```

</details>

The `Ops` column describe which operations are supported for each source.

### Token management

The `fetiche-sources`  crate has some support for token caching to avoid getting a fresh token for each call.  
The `list tokens` sub-command will show you the available tokens. These are per-identity tokens.

<details>
<summary>acutectl list tokens</summary>

```text
acutectl/0.11.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
CLI utility to fetch data.

Listing all tokens:
╭───────────────────────────────────────────────────┬───────────────────────────────────╮
│ Path                                              │ Created at                        │
├───────────────────────────────────────────────────┼───────────────────────────────────┤
│ asd_default_token-some.user@eurocontrol.int       │ 2023-05-31 20:31:43.027646800 UTC │
│ asd_default_token-ollivier.robert@eurocontrol.int │ 2023-05-24 09:17:44.891997300 UTC │
╰───────────────────────────────────────────────────┴───────────────────────────────────╯
```

</details>

### DB Import (incomplete)

The `acutectl import` sub-command will also use another one called `dbfile.hcl`  located in the same directory.

Here is an example of `dbfile.hcl`:

<details>
<summary>`dbfile.hcl`</summary>

```hcl
version = 1

db "local" {
  type   = sqlite
  format = "dronepoint"
  file   = "sqlite:///var/db/adsb.sqlite"
}

db "next" {
  type   = pgsql
  format = "opensky"
  url    = "pgsql://mydbserver:5432/adsb-data"
}

db "time" {
  type  = influxdb
  url   = "http://localhost:8600"
  token = "NOT DISCLOSED HERE"
}
```

> NOTE:  This will almost certainly change in the near future when I get to implement the DB import.

</details>

[Parquet]: https://parquet.apache.org/docs/file-format/
