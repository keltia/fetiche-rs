<!-- omit in TOC -->

# fetiche-rs

<img src="docs/fetiche-rs-icon.jpg" alt="Fetiche Logo from Kirikou movie" />

> **FETICHE: Framework to import/fetch/transform various aeronautical data**

[![Build status](https://github.com/keltia/fetiche-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/keltia/fetiche-rs/actions/workflows/rust.yml)
[![Build status (develop)](https://github.com/keltia/fetiche-rs/actions/workflows/develop.yml/badge.svg)](https://github.com/keltia/fetiche-rs/actions/workflows/develop.yml)
[![dependency status](https://deps.rs/repo/github/keltia/fetiche-rs/status.svg)](https://deps.rs/repo/github/keltia/fetiche-rs)
[![Docs](https://img.shields.io/docsrs/dmarc-rs)](https://docs.rs/fetiche-rs)
[![GitHub release](https://img.shields.io/github/release/keltia/dmarc-rs.svg)](https://github.com/keltia/fetiche-rs/releases/)
[![GitHub issues](https://img.shields.io/github/issues/keltia/fetiche-rs.svg)](https://github.com/keltia/fetiche-rs/issues)
[![fetiche-rs: 1.85+]][Rust 1.85]
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

This is now divided into different crates with libraries (`fetiche-common` , `fetiche-engine`, `fetiche-formats`, and
`fetiche-macros`) shared by the binary crates (`acutectl`, `opensky-history` and now `process-data`).

Binary crates include command-line utilities (`acutectl` and `opensky-history`) to perform import from a file or
fetch data from different sites. There is now `process-data` which include several tasks aimed at gathering statistics
and metrics about our drone and flight data.

`acutectl` is the main data fetching utility, which uses the `fetiche-engine` and `fetiche-formats` crates to provide
a single interface to multiple sources (`fetiche-sources` is now integral part of the engine).

`process-data`  works on the drone and flight data through [Clickhouse] and does various SQL-backed procedures to
gather and calculates metrics including distances (2D and 3D).

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

If you want to use [jujutsu] alongside git, it is also very easy:

```shell
$ git clone https://github.com/keltia/fetiche-rs
$ cd fetiche-rs
$ jj git init --colocate
$ jj b track main@origin develop@origin
$ cargo install --path .
```

See [jujutsu] documentation on how to use it instead of git.

> NOTE: I do use [jujustu] myself now.

## Usage

For the moment, there are three binaries called `acutectl` (with `.exe` on Windows), `opensky-history` and
`process-data`.
The former is used to fetch data into their native format (csv, json). It uses `fetiche-engine` for all the code related
to accessing, authenticating and fetching data in various ways.

All the commands are described in more detail in the [acutectl README.md](acutectl/README.md),
[opensky-history README.md](opensky-history/README.md) and [process-data](process-data/README.md) files.

## `fetiched` (managed in the `fetiched` crate)

On UNIX systems, there is a new command called `fetiched`. It is a daemon running the latest engine, detaching itself
from the terminal and accepting requests through a [GRPC] interface. The Windows version will have to be run from a
specific terminal with the `serve` command.

In the near future, `fetiched` is evolving into an Actor-based subsystem (using [ractor]) to manage
orchestration between the internal modules. We do have an engine actor, a configuration actor, etc.

More details in the specific [Fetiched README.md](fetiched/README.md) and [Engine README](engine/README.md).

> NOTE: This is still WIP and most of it is already in `fetiche-engine`.  [ractor] is used right now.

### Data Model

Each source has its own data model which complicates things, apart from [ASTERIX] with Cat129 for drone data, each
company/service provider use their own data model.

See the `fetiche-formats` crate for more details.

### Cargo Features

Some of the crates like `fetiche-formats` have specific features for different manufacturers.  
It helps reduce compilation time. See the specific `Cargo.toml` in each.

There is one feature enabled by default in the engine, called `privacy`. This is for truncating the drone ID to a
less-easily identifiable value. See `Cargo.toml` for this.

This is intentionally *not* a run-time option but a compile-time one.

## MSRV

The Minimum Supported Rust Version is *1.85* due to the `async traits` used through `fetiche-engine` and for
Clickhouse connections in `process-data`. We are now using Edition 2024.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
    - Powershell (preferred)
    - cmd.exe
    - [Nushell]

## TODO

Here are some of the things I've been working on. Some of these are registered as issues on [GitHub issues].

- Add more tests and benchmarks.
- convert the opensky code to use [actors](https://en.wikipedia.org/wiki/Actor_model).
- add a feeder that dispatches different events in the Avionix flow into separate AMQP queues.

Upcoming refactors:

- Rewrite Engine to be Actor-based. (ONGOING)
- Remove `Cat21` and all its derivatives (`Adsb21`, etc.). We do not use this anymore.

Uncertain:

- build `fetiched` as the core daemon and making all other talk to it through gRPC.
- link to HashiCorp Vault for storing credentials and tokens
- support for Safesky for ADS-B data

See [the issues](https://github.com/keltia/fetiche-rs/issues/) for more details.

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

[fetiche-rs: 1.85+]: https://img.shields.io/badge/Rust%20version-1.85%2B-lightgrey

[Mozilla]: https://mozilla.org/

[Opensky]: https://www.opensky-network.org/

[Parquet]: https://parquet.apache.org/

[RUST]: https://www.rust-lang.org/

[Rust 1.78]: https://blog.rust-lang.org/2024/05/02/Rust-1.78.0.html

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

[jujutsu]: https://jj-vcs.github.io/jj/latest/
