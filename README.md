<!-- omit in TOC -->

# fetiche-rs

> **FETICHE: Framework to import/fetch/transform various aeronautical data**

[![Build status](https://github.com/keltia/fetiche-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/keltia/fetiche-rs/actions/workflows/rust.yml)
[![Buildstatus (develop)](https://github.com/keltia/fetiche-rs/actions/workflows/develop.yml/badge.svg)](https://github.com/keltia/fetiche-rs/actions/workflows/develop.yml)
[![Docs](https://img.shields.io/docsrs/dmarc-rs)](https://docs.rs/fetiche-rs)
[![GitHub release](https://img.shields.io/github/release/keltia/dmarc-rs.svg)](https://github.com/keltia/fetiche-rs/releases/)
[![GitHub issues](https://img.shields.io/github/issues/keltia/fetiche-rs.svg)](https://github.com/keltia/fetiche-rs/issues)
[![fetiche-rs: 1.56+]][Rust 1.56]
[![SemVer](https://img.shields.io/badge/semver-2.0.0-blue)](https://semver.org/spec/v2.0.0.html)
[![License](https://img.shields.io/crates/l/mit)](https://opensource.org/licenses/MIT)

Licensed under the [MIT](LICENSE) license.

1. [About](#about)
2. [History](#history)
3. [Installation](#installation)
4. [Structure](#structure)
5. [Usage](#usage)
6. [Supported formats](#formats)
7. [MSRV](#msrv)
8. [TODO](#todo)
9. [Contributing](#contributing)

## About

**Fetiche** is a framework with a set of libraries and utilities dealing with various data formats and import/conversion
utilities for Aeronautical data about drones and aircraft.

This is now divided into different crates with libraries (`fetiche-engine`, `fetiche-formats`, `fetiche-sources`) shared
by the binary crates (`acutectl` and `cat21conv`).

Binary crates include a command-line utility called `acutectl` to perform import from a file or fetch data from
different sites. This program has been enhanced to cover both file and network input and as well to support more
input formats.

In a second phase, `acutectl` will be used to import ADS-B data into tables on a MySQL/MariaDB/Postgres/InfluxDB
database, and replace both `cat21conv`.

## History

For the **ACUTE** Project, Marc Gravis wrote the Shell script `aeroscope.sh` in 2021 to fetch data from the aeroscope
server in EIH and transform it into a pseudo-Cat21 CSV file using the same field as the Category 21 from [ASTERIX]
Specifications. It uses `wget(1)` to fetch data and `jq(1)` and `awk(1)`  to transform it.

It works fine, but it is a bit fragile, has some hardcoded paths & filenames. This is an attempt at rewriting it
in [RUST], a fast and safe language defined in 2010 by [Mozilla]. It has been since evolved into a set of libraries and
binaries.

It is now known as the **Fetiche** surveillance framework.

## Installation

It might be available at some point as crates on [Crates.io]  but for the moment just as a private repository on
[GitHub]. Installation can be done either through a compiled binary for your platform or by cloning the repo and
compiling.

### Cargo Features

There is one feature enabled by default, called `privacy`. This is for truncating the drone ID to a less-easily
identifiable value. See `Cargo.toml` for this.

This is intentionally *not* a run-time option but a compile-time one.

## Usage

For the moment, there is only one binary called `acutectl` (with `.exe` on Windows). It can be used to fetch data into
their native format (csv, json) or import said data into a database. It can import both drone and ADS-B data depending
on the source's declared type in `sources.hcl`. Right now, `acutectl` use blocking HTTP calls and is not using any
`async` features.

However, while working on streaming support for Opensky, I have been experimenting with [tokio] for async support and
`acutectl` might eventually become fully-async. It does help for some stuff including signal (read ^C) support.

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

As seen, there are different sub-commands. You can use `acutectl help <sub-command>`  to get description of the
different parameters.

The `completion` keyword can be used to generate completion scripts for various shells including `zsh` on UNIX
and `powershell` on Windows.

Credentials are stored into the `acutectl` configuration file, located in the same directory but named, as one
can expect, `config.hcl`.

<details>
<summary>`config.hcl`</summary>

```text
version = 1

site "local" {
  auth = {
    username = "aeroscope"
    password = "NOPE"
    token    = "/login"
  }
}

site "big.site.aero" {
  auth = {
    username = "SOMEONE"
    password = "HIDDEN"
    token = "/auth"
  }
}

site "opensky" {
  auth = {
    login    = "someone"
    password = "SECRET" 
  }
}

site "safesky" {
  auth {
    api_key = "FOOBAR"
  }
}

```

</details>

If you are just giving the utility a file, you must specify the input format with the `-F/--format` option.

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

## Structure and Design

`Fetiche` has 3 main component so far:

### Engine (managed in the `fetiche-engine` crate)

As the name implies, this is the heart of the `Fetiche` framework. It is a fully-threaded engine, with one thread per
job and each task
has a number of threads inside. It use a pipeline design that ensure that every stage has input from the previous one
and send its own
output to the next one. Some stage/tasks are filters (`Convert`) and some are either consumer or producer (
notably `Fetch` and `Stream`).

This allows for filters to be inserted for conversion and in the future for DB export as well.

The current tasks defined are:

- `Nothing`
- `Message`
- `Copy`
- `Convert`
- `Fetch`
- `Stream`

I think it is more flexible to work within the framework of the engine.

> NOTE: this is a fast-changing WIP.

### Formats (managed in the `fetiche-formats` crate)

This crate implement the various data models used by the different sources. Included are two [ASTERIX] formats --
generic `Cat21` and drone-specific `Cat129` -- and formats used by different data providers like [Opensky] or [ASD].
This library implement some methods of conversion between some of these formats.

The default input format is the one used by the Aeroscope from ASD, but it will soon support the format used
by [Opensky] site. There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

To displayed currently supported formats, use `acutectl list formats`:

<details>
<summary>acutectl list formats</summary>

```text
acutectl/0.11.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
CLI utility to fetch data.

List all formats:
┌───────────┬───────┬───────────────────────────────────────────────────────────┐
│ Name      │ Type  │ Description                                               │
├───────────┼───────┼───────────────────────────────────────────────────────────┤
│ aeroscope │ drone │ Data extracted from the DJI Aeroscope antenna.            │
│           │       │ Source: ASD -- URL: https://airspacedrone.com/            │
├───────────┼───────┼───────────────────────────────────────────────────────────┤
│ asd       │ drone │ Data gathered & consolidated by ASD.                      │
│           │       │ Source: ASD -- URL: https://airspacedrone.com/            │
├───────────┼───────┼───────────────────────────────────────────────────────────┤
│ cat129    │ drone │ Flattened ASTERIX Cat129 data for Drone data.             │
│           │       │ Source: ECTL -- URL: https://www.eurocontrol.int/asterix/ │
├───────────┼───────┼───────────────────────────────────────────────────────────┤
│ cat21     │ adsb  │ Flattened ASTERIX Cat21 data for ADS-B.                   │
│           │       │ Source: ECTL -- URL: https://www.eurocontrol.int/asterix/ │
├───────────┼───────┼───────────────────────────────────────────────────────────┤
│ opensky   │ adsb  │ Data coming from the Opensky site, mostly ADS-B.          │
│           │       │ Source: Opensky -- URL: https://opensky-network.org/      │
├───────────┼───────┼───────────────────────────────────────────────────────────┤
│ safesky   │ adsb  │ Data coming from the Safesky site, mostly ADS-B.          │
│           │       │ Source: Safesky -- URL: https://www.safesky.app/          │
└───────────┴───────┴───────────────────────────────────────────────────────────┘
```

The reason for the different categories is to give the engine a hint on how to process the data. Drone data will be
transformed into our `DronePoint` and `Journey` types for post-processing.

</details>

### Sources (managed in the `fetiche-sources` crate)

The configuration for the different sources of data is handled by the `fetiche-source` crate in [HCL] file
format. Note that it is mainly used to avoid hard-coding some parameters like username and API URLs. Adding an entry
in that file does not mean support except if it is a variation on a known source.

On UNIX systems, it is located in `$HOME/.config/drone-utils/sources.hcl` and in `%LOCALAPPDATA%\DRONE-UTILS` on
Windows.

The current config file version is 4. This is where all the URL for the parts of each API are defined, which routes are
available, the default data model etc.

<details>
<summary>sources.hcl</summary>

```hcl
version = 4

site "local" {
  features = ["fetch"]
  type     = "drone"
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  routes   = {
    get = "/drone/get"
  }
}

site "big.site.aero" {
  features = ["fetch"]
  type     = "drone"
  format   = "asd"
  base_url = "https://api.site.aero"
  routes   = {
    get = "/api/journeys/filteredlocations/json"
  }
}

site "opensky" {
  features = ["fetch", "stream"]
  type     = "adsb"
  format   = "opensky"
  base_url = "https://opensky-network.org/api"
  routes   = {
    get = "/states/own"
  }
}

site "safesky" {
  features = ["fetch"]
  type     = "adsb"
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  routes   = {
    get = "/v1/beacons"
  }
}
```

</details>

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

### Data Model

Each source has its own data model which complicates things, apart from [ASTERIX] with Cat129 for drone data, each
company/service provider use their own data model. To ease managing drone data, I have defined `DronePoint` as a common
data model (extracted from the data sent by [ASD] with some fields with different types -- like actual `f32` instead of
the string format) and real timestamp. These can be grouped into a `Journey` type which is a state vector with all the
points in the trajectory.

See the `fetiche-formats`  crate for more details.

## MSRV

The Minimum Supported Rust Version is *1.56* due to the 2021 Edition.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
  * cmd.exe
  * Powershell (preferred)

## TODO

Here are some of the things I've been working on. Some of these are registered as issues on [GitHub issues].

- ~~support more parameters (like dates, etc.)~~
- ~~fetch and analyse from Aeroscope~~
- ~~fetch and analyse from Asd~~
- ~~divide into crates for sharing more code.~~
- ~~use a common data model for drone data~~
- ~~Support for Opensky (same)~~
- ~~make `acutectl` use `fetiche-engine` instead of its own `task.rs`~~.
- ~~add streaming support for sources like opensky~~.
- ~~rename `drone-utils` into the more proper `fetiche-rs`~~.
- merge `import-adsb` and `cat21conv` into `acutectl`.
- rename `acutectl` into `fetichectl`.
- link to HashiCorp Vault for storing credentials and tokens
- ~~alternatively caching tokens (like ASD ones) locally~~
- Add more tests & benchmarks.
- support for Safesky for ADS-B data
- Support for Sherlock formats and access methods
- Data formats conversion framework
- Multicast output?

## Configuration History

The `sources.hcl` configuration file is versioned to avoid incompatibilities.

- v1 was the original version with the `Sites` struct
- In v2 `Sites`  was renamed into `Sources` to reflect evolution
- In v3 the `type`  keyword was added to the `Site` definition
- In v4 the `features` keyword was added to indicate what is supported between `fetch` and `stream`.

The `formats.hcl` add metadata about all supported formats.

- v1 was generic
- v2 added the datatype for each format

The `dbfile.hcl` list possible database connections.

- v1 as the original design, to be evolved.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for some simple rules.

I use Git Flow for this package so please use something similar or the usual GitHub workflow.

1. Fork it [here](https://github.com/keltia/fetiche-rs/fork)
2. Checkout the develop branch (`git checkout develop`)
3. Create your feature branch (`git checkout -b my-new-feature`)
4. Commit your changes (`git commit -am 'Add some feature'`)
5. Push to the branch (`git push origin my-new-feature`)
6. Create a new Pull Request

[ASD]: https://eur.airspacedrone.com/

[ASTERIX]: https://www.eurocontrol.int/asterix/

[fetiche-rs: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey

[Mozilla]: https://mozilla.org/

[Opensky]: https://www.opensky-network.org/

[RUST]: https://www.rust-lang.org/

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[Safesky]: https://safesky.app/

[HCL]: https://developer.hashicorp.com/terraform/language

[GitHub issues]: https://github.com/keltia/fetiche-rs/issues

[tokio]: https://crates.io/crates/tokio
