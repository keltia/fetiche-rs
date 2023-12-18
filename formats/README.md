<!-- omit in TOC -->

# fetiche-format

> **Library to import/fetch/transform between various aeronautical data formats**

## About

This crate contains only the code for the different input file formats supported by `acutectl` as part of the Fetiche
framework.

- Aeroscope - basic format coming straight from the DJI antenna
- Asd - The JSON & CSV format from `airspacedrones.com`.
- [Opensky] - ADS-B data from the Opensky network of probes
- [ASTERIX] Cat21 & Cat129 (the flattened CSV-based versions) and the new Adsb21, a trimmed-down version of Cat21 for
  ADS-B data
- [Avionix] - another variation on a flattened Cat21-like format
- Safesky (WIP)

There are also so-called output formats (or containers) when you fetch data and write it into files:

- Plain CSV
- Annotated CSV, like in InfluxDB
- [Parquet], a columnar compressed data format from Apache

### Features

There is one feature enabled by default, called `privacy`. This is for truncating the drone ID to a less-easily
identifiable value. See `Cargo.toml` for this.

This is intentionally *not* a run-time option but a compile-time one.

## Formats

The default format is the one used by the Aeroscope from ASD, but it will soon support the format used by [Safesky]
site. There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

These are described in the `src/format/aeroscope.rs`, `src/format/asd.rs` and `src/format/safesky.rs` files. There are
also transformations in each case when converting into our CSV-based Cat21-like format.

### Cat21

Our own Cat21-like format is named because it uses the field names coming from the [ASTERIX] specifications (although
everything is flat in a csv so enums are flattened as well). See the files in `src/asterix`  for the description.

### Adsb21

This is a trimmed-down version of `Cat21` which include only the fields we currently use when we import ADS-B data from
either [Opensky] or [Flightaware] sources.

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
- ~~Support for [Opensky]~~
- ~~Support for [Flightaware]~~
- ~~Support for Apache Parquet~~
- support for Safesky
- Add more tests & benchmarks.

[ASD]: https://airspacedrone.com/

[ASTERIX]: https://www.eurocontrol.int/asterix/

[Mozilla]: http://mozilla.org/

[RUST]: https://www.rust-lang.org/

[fetiche-rs: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[Safesky]: https://safesky.app/

[TOML]: https://github.com/naoina/toml/

[Opensky]: https://opensky-network.org/

[Avionix]: http://www.avionix.pl/

[Flightaware]: https://www.flightaware.com/firehose/documentation

[Parquet]: https://parquet.apache.org/docs/file-format/
