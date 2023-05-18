<!-- omit in TOC -->

# import-adsb

> **Utility to import aeronautical data into a database**

## About

This rate contains only the code for the different input file formats supported by `cat21conv`:

- Aeroscope
- Asd
- Opensky
- Safesky

## Configuration

`import-adsb` use a configuration file along with the `config.toml`  from the `format-specs` library, `dbfile.toml`.

<details>
<summary>dbfile.toml</summary>

```toml
default = "none"

[db.sqlite]
name = "local"
path = "./local.sqlite"

[db.mysql]
name = "drone"
url = "mysql://example.net:3306/drones"
```

</details>

## Formats

The default format is the one used by the Aeroscope from ASD, but it will soon support the format used by [Safesky]
site. There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

These are described in the `fetiche-formats` crate. There are also transformations in each case when converting into our
CSV-based Cat21-like format.

## Usage

```text
CLI utility to import ADS-B data.

Usage: import-adsb [OPTIONS] <COMMAND>

Commands:
  create-db
  import
  help       Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  configuration file
  -d, --dbfile <DBFILE>  DB connection file
  -D, --debug            debug mode
  -F, --format <FORMAT>  Format must be specified if looking at a file
  -S, --site <SITE>      Site to fetch data from
  -v, --verbose...       Verbose mode
  -V, --version          Display utility full version
  -h, --help             Print help information
```

## MSRV

The Minimum Supported [Rust] Version is *1.56* due to the 2021 Edition.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
  * cmd.exe
  * Powershell

## TODO

[ASD]: https://eur.airspacedrone.com/

[ASTERIX]: https://www.eurocontrol.int/asterix/

[Mozilla]: http://mozilla.org/

[RUST]: https://www.rust-lang.org/

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[Safesky]: https://safesky.app/

[TOML]: https://github.com/naoina/toml/
