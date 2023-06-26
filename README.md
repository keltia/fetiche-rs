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
2. [Installation](#installation)
3. [Structure](#structure)
4. [Usage](#usage)
5. [Supported formats](#formats)
6. [MSRV](#msrv)
7. [TODO](#todo)
8. [Contributing](#contributing)

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
their native format (csv, json) or import said data into a database.  It uses the `fetiche-sources` for all the code 
related to accessing, authenticating and fetching data in various ways.  

Right now, `acutectl` use blocking HTTP calls and is not using any
`async` features.

However, while working on streaming support for Opensky, I have been experimenting with [tokio] for async support and
`acutectl` might eventually become fully-async. It does help for some stuff including signal (read ^C) support.

All the commands are described in more details in the [acutectl README.md](acutectl/README.md).

## Structure and Design

`Fetiche` has 3 main component so far:

### Engine (managed in the `fetiche-engine` crate)

As the name implies, this is the heart of the `Fetiche` framework. It is a fully-threaded engine, with one thread per
job and each task has a number of threads inside. It uses a pipeline design that ensure that every stage has input from
the previous one and send its own output to the next one. Some stage/tasks are filters (`Convert`) and some are either
consumer or producer (notably `Fetch`, `Stream` and `Store`).

This allows for filters to be inserted for conversion and in the future for DB export as well.

More information on its internal design in [Engine README.md](engine/README.md).

> NOTE: this is a fast-changing WIP.

### Formats (managed in the `fetiche-formats` crate)

This crate implement the various data models used by the different sources. Included are two [ASTERIX] formats --
generic `Cat21` and drone-specific `Cat129` -- and formats used by different data providers like [Opensky] or [ASD].
This library implement some methods of conversion between some of these formats.

The default input format is the one used by the Aeroscope from ASD, but it will soon support the format used
by [Opensky] site. There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

More details in the [Formats README.md](formats/README.md).

### Sources (managed in the `fetiche-sources` crate)

The configuration for the different sources of data is handled by the `fetiche-source` crate in [HCL] file
format. Note that it is mainly used to avoid hard-coding some parameters like username and API URLs. Adding an entry
in that file does not mean support except if it is a variation on a known source.

You are not really supposed to edit this file.

More details in the specific [Sources README.md](sources/README.md).

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
- ~~Data formats conversion framework~~
- ~~caching tokens (like ASD ones) locally~~
- ~~merge `import-adsb` and `cat21conv` into `acutectl`~~.
- ~~Add a `Store` module to handle long-running jobs and their output.~~
- rename `acutectl` into `fetichectl`.
- link to HashiCorp Vault for storing credentials and tokens
- Add more tests & benchmarks.
- support for Safesky for ADS-B data
- Support for Sherlock formats and access methods
- Multicast output?

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
