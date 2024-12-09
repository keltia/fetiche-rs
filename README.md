<!-- omit in TOC -->

# fetiche-rs

[![Fetiche Logo](docs/fetiche-rs-icon.jpg)]

> **FETICHE: Framework to import/fetch/transform various aeronautical data**

[![Build status](https://github.com/keltia/fetiche-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/keltia/fetiche-rs/actions/workflows/rust.yml)
[![Build status (develop)](https://github.com/keltia/fetiche-rs/actions/workflows/develop.yml/badge.svg)](https://github.com/keltia/fetiche-rs/actions/workflows/develop.yml)
[![dependency status](https://deps.rs/repo/github/keltia/fetiche-rs/status.svg)](https://deps.rs/repo/github/keltia/fetiche-rs)
[![Docs](https://img.shields.io/docsrs/dmarc-rs)](https://docs.rs/fetiche-rs)
[![GitHub release](https://img.shields.io/github/release/keltia/dmarc-rs.svg)](https://github.com/keltia/fetiche-rs/releases/)
[![GitHub issues](https://img.shields.io/github/issues/keltia/fetiche-rs.svg)](https://github.com/keltia/fetiche-rs/issues)
[![fetiche-rs: 1.56+]][Rust 1.56]
[![SemVer](https://img.shields.io/badge/semver-2.0.0-blue)](https://semver.org/spec/v2.0.0.html)
[![License](https://img.shields.io/crates/l/mit)](https://opensource.org/licenses/MIT)

Licensed under the [MIT](LICENSE) license.

1. [About](#about)
2. [Installation](#installation)
3. [Usage](#usage)
4. [Structure](#structure-and-design)
5. [MSRV](#msrv)
6. [TODO](#todo)
7. [Contributing](#contributing)

## About

**Fetiche** is a framework with a set of libraries and utilities dealing with various data formats and import/conversion
utilities for Aeronautical data about drones and aircraft.

This is now divided into different crates with libraries (`fetiche-engine`, `fetiche-formats`, `fetiche-sources`,
`fetiche-common` and `fetiche-macros`) shared by the binary crates (`acutectl`, `opensky-history` and now
`process-data`).

Binary crates include command-line utilities (`acutectl` and `opensky-history`) to perform import from a file or
fetch data from different sites. There is now `process-data` which include several tasks aimed at gathering statistics
and metrics about our drone and flight data.

`acutectl` is the main data fetching utility, relaying the `fetiche-sources` and `fetiche-formats` crates to provide
a single interface to multi sources.

`process-data`  works on the drones and flights data through [Clickhouse] and does various SQL-backed procedures to
gather and calculates metrics including distances (2D and 3D).

`adsb-to-parquet` is a temporary converter between the CSV files we receive the ADS-B data into compressed parquet
files. As my patch to improve [bdt]  has been merged, `bdt` is now used instead.

`opensky-history` is for retrieving historical data from [Opensky]. This access is managed through an SSH-based shell to
the Impala database. This is for everything older than 1h of real-time data which does complicate things. This utility
use the [pyopensky] Python module (embedded through the `inline-python` crate). There is also a pure python script that
does the same in `scripts/`.

## Installation

It might be available at some point as crates on [Crates.io]  but for the moment just as a public repository on
[GitHub]. Installation can be done either through a compiled binary for your platform or by cloning the repo and
compiling.

You should be able to compile Fetiche by simply:

```shell
$ git clone https://github.com/keltia/fetiche-rs
$ cd fetiche-rs
$ cargo install --path .
```

to compile and install `acutectl` and `process-data`.

## Usage

For the moment, there are 3 binaries called `acutectl` (with `.exe` on Windows), `opensky-history` and `process-data`.
The former is used to fetch data into their native format (csv, json). It uses `fetiche-engine` for all the code related
to accessing, authenticating and fetching data in various ways.

Right now, `fetiche-engine` use blocking HTTP calls and is not using any `async` features.

However, while working on streaming support for Opensky, I have been experimenting with [tokio] for async support and
`acutectl` might eventually become fully-async. It does help for some stuff including signal (read ^C) support.

All the commands are described in more details in the [acutectl README.md](acutectl/README.md),
[opensky-history README.md](opensky-history/README.md) and [process-data](process-data/README.md) files.

## `fetiched` (managed in the `fetiched` crate)

On UNIX systems, there is a new command called `fetiched`. It is a daemon running the latest engine, detaching itself
from the terminal and accepting requests through an [GRPC] interface. The Windows version will have to be run from a
specific terminal with the `serve` command.

In the near future, `fetiched` is evolving into an Actor-based subsystem (using [Actix] or [ractor]) to manage
orchestration between the internal modules. We do have an engine actor, a configuration actor, etc.

More details in the specific [Fetiched README.md](fetiched/README.md).

> NOTE: This is still WIP

### Data Model

Each source has its own data model which complicates things, apart from [ASTERIX] with Cat129 for drone data, each
company/service provider use their own data model.

See the `fetiche-formats` crate for more details.

### Cargo Features

Some of the crates like `fetiche-formats` and `fetiche-sources` have specific features for different manufacturers.  
It helps reduces compilation time. See the specific `Cargo.toml` in each.

There is one feature enabled by default, called `privacy`. This is for truncating the drone ID to a less-easily
identifiable value. See `Cargo.toml` for this.

This is intentionally *not* a run-time option but a compile-time one.

## MSRV

The Minimum Supported Rust Version is *1.56* due to the 2021 Edition.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
    - Powershell (preferred)
    - cmd.exe
    - [Nushell]

## TODO

Here are some of the things I've been working on. Some of these are registered as issues on [GitHub issues].

- Add more tests & benchmarks.
- convert the opensky code to use actors.
- add a feeder that dispatch different events in the avionix flow into separate AMQP queues.

Done:

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
- ~~Retrieve historical data from the [Opensky] site.~~
- ~~Support for Flightaware AeroAPI and Firehose.~~
- ~~Apache Parquet as output format.~~
- ~~Migrate from the embedded [DuckDB] to a proper server-based DB [Clickhouse]~~
- ~~[Polars] instead of [Datafusion] to simplify? We are not using (nor plan to) all the datafusion features.~~
- ~~Integration of Avionix and Thales/Senhive antennas in `formats` and `sources` when we have docs.~~
- ~~Convert the current Avionix streaming code to the actor-based one in Senhive using [ractor].~~

Upcoming refactors:

- Remove `Cat21` and all its derivatives (`Adsb21`, etc.). We do not use this anymore.
- Merge back `fetiched/src/engine`  into the main `engine`, which imply using actors for the main engine too.
- Merge `fetiche-formats`  and `fetiche-sources` as they are completely linked and dependent. Maybe create a more
  general "Plugin" framework.

Uncertain:

- build `fetiched` as the core daemon and making all other talk to it through gRPC.
- link to HashiCorp Vault for storing credentials and tokens
- support for Safesky for ADS-B data
- Support for Sherlock formats and access methods
- `acutectl` and `process-data` could be merged at some point, not sure if it is useful.

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

[Actix]: https://actix.rs/

[ASTERIX]: https://www.eurocontrol.int/asterix/

[fetiche-rs: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey

[Mozilla]: https://mozilla.org/

[Opensky]: https://www.opensky-network.org/

[Parquet]: https://parquet.apache.org/

[RUST]: https://www.rust-lang.org/

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[Safesky]: https://safesky.app/

[HCL]: https://developer.hashicorp.com/terraform/language

[GitHub issues]: https://github.com/keltia/fetiche-rs/issues

[tokio]: https://crates.io/crates/tokio

[GRPC]: https://en.wikipedia.org/wiki/GRPC

[pyopensky]: https://pypi.org/project/pyopensky/

[Nushell]: https://nushell.sh/

[DuckDB]: https://duckdb.org/

[Clickhouse]: https://clickhouse.com/

[bdt]: https://github.com/datafusion-contrib/bdt

[Polars]: https://pola.rs/

[ractor]: https://crates.io/crates/ractor
