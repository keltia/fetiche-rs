<!-- omit in TOC -->
# drone-utils

> **Library to import/fetch/transform various aeronautical data into Cat21-like format**

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
2. [Installation](#installation)
3. [Usage](#usage)
4. [Supported formats](#formats)
5. [MSRV](#msrv)
6. [TODO](#todo)
7. [Contributing](#contributing)

## About

For the **ACUTE** Project, Marc Gravis wrote the Shell script `aeroscope.sh` in 2021 to fetch data from the aeroscope
server in EIH and transform it into a pseudo-Cat21 CSV file using the same field as the Category 21 from [ASTERIX]
Specifications. It uses `wget(1)` to fetch data and `jq(1)` and `awk(1)`  to transform it.

It works fine, but it is a bit fragile, has some hardcoded paths & filenames. This is an attempt at rewriting it
in [RUST], a fast and safe language defined in 2010 by [Mozilla] and currently evolving with 2 releases a year.

This library supports different formats and access methods and include a command-line utility called `cat21conv` to
perform import from a file, fetching data from different sites. This program has been enhanced to cover both file and
network input and as well to support more input formats.

## Installation

## Usage

```text
$ cat21conv
CLI utility to convert Aeroscope data into Cat21 CSV.

Usage: cat21conv [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file

Options:
  -B, --begin <BEGIN>      Start the data at specified date (optional)
  -c, --config <CONFIG>    configuration file
  -D, --debug              debug mode
  -E, --end <END>          End date (optional)
  -F, --format <FORMAT>    Format must be specified if looking at a file
  -o, --output <OUTPUT>    Output file
  -S, --site <SITE>        Site to fetch data from
      --today              We want today only
  -v, --verbose <VERBOSE>  Verbose mode
  -V, --version            Display utility full version
  -h, --help               Print help information
```

The `cat21conv` utility uses a configuration file in the [TOML] file format.

On UNIX, it is located in `$HOME/.config/drone-utils/config.toml` and in `%LOCALAPPDATA%\DRONE-UTILS` on Windows.

There are only a few parameters for now, the most important one being the credentials for authenticate against the
network endpoint. You can specify the different network endpoints:

```toml
default = "none"

[sites.someplace]

format = "aeroscope"
base_url = "http://127.0.0.1:2400"
token = "/login"
login = "SOMETHING"
password = "NOPE"
get = "/drone/get"

[sites.else]

format = "safesky"
base_url = "http://example.net:2400"
token = "/auth"
login = "USER"
password = "MAYBE"
get = "/foo"

[sites.nope]

format = "safesky"
base_url = "https://kansas.example.net:3000"
get = "/somewhere/over/the/rainbow"
```

As you can see, there are sites that require you to supply a login & password and others which don't.

The site name is supplied through the `-S/--site` option. If you are just giving the utility a file, you must specifiy
the input format with the `-F/--format` option.

## Formats

The default format is the one used by the Aeroscope from ASD, but it will soon support the format used by [Safesky]
site.  
There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

These are described in the `src/format/aeroscope.rs`, `src/format/asd.rs` and `src/format/safesky.rs` files. There are
also
transformations in each case when converting into our CSV-based Cat21-like format.

### Cat21

Our own Cat21-like format is named because it uses the field names coming from the [ASTERIX] specifications (although
everything is flat in a csv so enums are flattened as well). See `src/format/mod.rs`  for the description.

## MSRV

The Minimum Supported Rust Version is *1.56* due to the 2021 Edition.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
  * cmd.exe
  * Powershell

## TODO

- ~~support more parameters (like dates, etc.)~~
- ~~fetch and analyse from Aeroscope~~
- ~~fetch and analyse from Asd~~
- Add more tests & benchmarks.
- support for Safesky

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
[TOML]: https://github.com/naoina/toml/
