<!-- omit in TOC -->

# drone-utils

> **Library to import/fetch/transform various aeronautical data**

[![Build status](https://github.com/keltia/drone-gencsv/actions/workflows/rust.yml/badge.svg)](https://github.com/keltia/drone-gencsv/actions/workflows/rust.yml)
[![Buildstatus (develop)](https://github.com/keltia/drone-gencsv/actions/workflows/develop.yml/badge.svg)](https://github.com/keltia/drone-gencsv/actions/workflows/develop.yml)
[![Docs](https://img.shields.io/docsrs/dmarc-rs)](https://docs.rs/drone-utils)
[![GitHub release](https://img.shields.io/github/release/keltia/dmarc-rs.svg)](https://github.com/keltia/drone-gencsv/releases/)
[![GitHub issues](https://img.shields.io/github/issues/keltia/drone-gencsv.svg)](https://github.com/keltia/drone-gencsv/issues)
[![drone-utils: 1.56+]][Rust 1.56]
[![SemVer](https://img.shields.io/badge/semver-2.0.0-blue)](https://semver.org/spec/v2.0.0.html)
[![License](https://img.shields.io/crates/l/mit)](https://opensource.org/licenses/MIT)

Licensed under the [MIT](LICENSE) license.

1. [About](#about)
2. [History](#history)
2. [Installation](#installation)
3. [Usage](#usage)
4. [Supported formats](#formats)
5. [MSRV](#msrv)
6. [TODO](#todo)
7. [Contributing](#contributing)

## About

This is a set of libraries and utilities dealing with various data formats and import/conversion utilities for
Aeronautical data about drones and aircraft.

This is now divided into 4 different crates with two libraries (`format-specs` and `sources`) shared by the binary
crates (`acutectl`, `cat21conv` and `import-adsb`).

These libraries support different formats (`format-specs`) and access methods (`sources`).

Binary crate include a command-line utility called `acutectl` to perform import from a file or fetching data from
different sites. This program has been enhanced to cover both file and network input and as well to support more
input formats.

In a second phase, `acutectl` will be used to import ADS-B data into tables on a MySQL/MariaDB/Postgres/InfluxDB
database, and replace both `cat21conv` and `import-adsb`.

## History

For the **ACUTE** Project, Marc Gravis wrote the Shell script `aeroscope.sh` in 2021 to fetch data from the aeroscope
server in EIH and transform it into a pseudo-Cat21 CSV file using the same field as the Category 21 from [ASTERIX]
Specifications. It uses `wget(1)` to fetch data and `jq(1)` and `awk(1)`  to transform it.

It works fine, but it is a bit fragile, has some hardcoded paths & filenames. This is an attempt at rewriting it
in [RUST], a fast and safe language defined in 2010 by [Mozilla] and currently evolving with 2 releases a year. It
has been since evolved into a set of libraries and binaries.

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
on the source's declared type in `sources.hcl`.

<details>
<summary>acutectl</summary>

```text
$ acutectl
CLI utility to fetch data.

Usage: acutectl [OPTIONS] <COMMAND>

Commands:
  completion  Generate Completion stuff
  fetch       Fetch data from specified site
  import      Import into InfluxDB
  list        Handle drone data
  help        Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  configuration file
  -D, --debug            debug mode
  -o, --output <OUTPUT>  Output file
  -v, --verbose...       Verbose mode
  -V, --version          Display utility full version
  -h, --help             Print help
```

</details>

As seen, there are different sub-commands. You can use `acutectl help <sub-command>`  to get description of the
different parameters.

The configuration for the different sources of data is handled by the `source` crate in [HCL] file format.

On UNIX, it is located in `$HOME/.config/drone-utils/source.hcl` and in `%LOCALAPPDATA%\DRONE-UTILS` on Windows.

There are only a few parameters for now, the most important one being the credentials for authenticate against the
network endpoint. You can specify the different network endpoints. The current config file version is 3 as the `type`
entry was added to the `Site` struct.

The `completion` keyword can be used to generate completion sciprts for various shells incl `zsh` and `powershell`.

The `acutectl import` sub-command will also use another one called `dbfile.hcl`  located in the same directory.

<details>
<summary>sources.hcl</summary>

```hcl
version = 3

site "local" {
  type     = "drone"
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth     = {
    login    = "SOMETHING"
    password = "NOPE"
    token    = "/login"
  }
  routes = {
    get = "/drone/get"
  }
}

site "big.site.aero" {
  type     = "drone"
  format   = "asd"
  base_url = "https://api.site.aero"
  auth     = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/api/security"
  }
  routes = {
    get = "/api/journeys/filteredlocations"
  }
}

site "opensky" {
  type     = "adsb"
  format   = "opensky"
  base_url = "https://opensky-network.org/api"
  auth     = {
    login    = "anyone"
    password = "NOPE"
  }
  routes = {
    get = "/state/own"
  }
}

site "safesky" {
  type     = "adsb"
  format   = "safesky"
  base_url = "https://public-api.safesky.app"
  auth     = {
    api_key = "foobar"
  }
  routes = {
    get = "/v1/beacons"
  }
}
```

As you can see, there are sites that require you to supply a login & password and others which don't.

If you are just giving the utility a file, you must specify the input format with the `-F/--format` option.
</details>

Here is an example of `dbfile.hcl`:

<details>
<summary>dbfile.hcl</summary>

```hcl
version = "1"

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

</details>

## Formats (managed in the `format-specs`  crate)

The default input format is the one used by the Aeroscope from ASD, but it will soon support the format used
by [Safesky] site. There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

These are described in the `format-specs/src/s/aeroscope.rs`, `format-specs/src/s/asd.rs`
and `format-specs/src/s/safesky.rs` files. There are also transformations in each case when converting into our
CSV-based Cat21-like format (DEPRECATED).

To displayed currently supported formats, use `acutectl list formats`:

```text
acutectl/0.2.0 by Ollivier Robert <ollivier.robert@eurocontrol.int>
CLI utility to fetch data.

List all formats:

aeroscope drone Data extracted from the DJI Aeroscope antenna.
                Source: ASD -- URL: https://airspacedrone.com/

asd       drone Data gathered & consolidated by ASD.
                Source: ASD -- URL: https://airspacedrone.com/

cat129    drone Flattened ASTERIX Cat129 data for Drone data.
                Source: ECTL -- URL: https://www.eurocontrol.int/asterix/

cat21     adsb  Flattened ASTERIX Cat21 data for ADS-B.
                Source: ECTL -- URL: https://www.eurocontrol.int/asterix/

opensky   adsb  Data coming from the Opensky site, mostly ADS-B.
                Source: Opensky -- URL: https://opensky-network.org/

safesky   adsb  Data coming from the Safesky site, mostly ADS-B.
                Source: Safesky -- URL: https://www.safesky.app/
```

### DronePoint & Journey

`DronePoint` is a common data model extracted from the data sent by [ASD] with some fields with different types (like
actual `f32` instead of the string format) and real timestamp. These can be grouped into a `Journey` type which is a
state vector with all the points in the trajectory.

## MSRV

The Minimum Supported Rust Version is *1.56* due to the 2021 Edition.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
  * cmd.exe
  * Powershell (preferred)

## TODO

- ~~support more parameters (like dates, etc.)~~
- ~~fetch and analyse from Aeroscope~~
- ~~fetch and analyse from Asd~~
- ~~divide into crates for sharing more code.~~
- ~~use a common data model for drone data~~
- ~~Support for Opensky (same)~~
- merge `import-adsb` and `cat21conv` into `acutectl`.
- link to HashiCorp Vault for storing credentials and tokens
- alternatively caching tokens (like ASD ones) locally
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

The `formats.hcl` add metadata about all supported formats.

- v1 was generic
- v2 added the datatype for each format

The `dbfile.hcl` list possible database connections.

- v1 as the original design, to be evolved.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for some simple rules.

I use Git Flow for this package so please use something similar or the usual GitHub workflow.

1. Fork it [here](https://github.com/keltia/drone-gencsv/fork)
2. Checkout the develop branch (`git checkout develop`)
3. Create your feature branch (`git checkout -b my-new-feature`)
4. Commit your changes (`git commit -am 'Add some feature'`)
5. Push to the branch (`git push origin my-new-feature`)
6. Create a new Pull Request

[ASD]: https://eur.airspacedrone.com/

[ASTERIX]: https://www.eurocontrol.int/asterix/

[Mozilla]: http://mozilla.org/

[RUST]: https://www.rust-lang.org/

[drone-utils: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[Safesky]: https://safesky.app/

[HCL]: https://developer.hashicorp.com/terraform/language
