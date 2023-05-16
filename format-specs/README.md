<!-- omit in TOC -->

# format-specs

> **Library to import/fetch/transform various aeronautical data into Cat21-like format**

## About

This rate contains only the code for the different input file formats supported by `cat21conv`:

- Aeroscope
- Asd
- Opensky
- Safesky

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
- Support for Opensky

[ASD]: https://eur.airspacedrone.com/

[ASTERIX]: https://www.eurocontrol.int/asterix/

[Mozilla]: http://mozilla.org/

[RUST]: https://www.rust-lang.org/

[drone-utils: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[Safesky]: https://safesky.app/

[TOML]: https://github.com/naoina/toml/
