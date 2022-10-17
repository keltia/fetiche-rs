<!-- omit in TOC -->
# drone-gencsv

> **Command-line utility to generate pseudo-Cat21 data from Aeroscope CSV**

Licensed under the [MIT](LICENSE)

1. [About](#about)
2. [Installation](#installation)
3. [Usage](#usage)
4. [MSRV](#msrv) 
5. [TODO](#todo)
6. [Contributing](#contributing)

## About

For the **ACUTE** Project, Marc Gravis wrote the Shell script `aeroscope.sh` in 2021 to fetch data from the aeroscope server in EIH and transform it into a pseudo-Cat21 CSV file using the same field as the Category 21 from [ASTERIX] Specifications.  It uses `wget(1)` to fetch data and `jq(1)` and `awk(1)`  to transform it.

It works fine but it is a bit fragile, has some hardcoded paths & filenames.  This is an attempt at rewriting it in [RUST], a fast and safe language defined in 2010 by [Mozilla] and currently evolving with 2 releases a year.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
    * cmd.exe
    * Powershell

## Installation

## Usage

```text
CLI utility to convert Aeroscope data into Cat21 CSV.

Usage: drone-gencsv [OPTIONS] [INPUT]

Arguments:
  [INPUT]  Input file

Options:
  -c, --config <CONFIG>      configuration file
  -D, --debug                debug mode
  -o, --output <OUTPUT>      Output file
  -P, --password <PASSWORD>  Optional password
  -U, --username <USERNAME>  Optional username for the server API
  -v, --verbose <VERBOSE>    Verbose mode
  -V, --version              Display utility full version
  -h, --help                 Print help information
```

The `drone-gencsv` utility uses a configuration file in the [TOML] file format.

On UNIX, it is located in `$HOME/.config/drone-gencsv/config.toml` and in `%LOCALAPPDATA%\DRONE-GENCSV` on Windows.

There are only a few parameters for now, the most important one being the credentials for authenticate against the Aeroscope API endpoint.  You can now specify the default probe set (and override it from the CLI):

```toml
base_url = "http://127.0.0.1:2400/"
login = "SOMETHING"
password = "NOPE"
```

## MSRV

The Minimum Supported Rust Version is 1.56 due to the 2021 Edition.

## TODO

- Add more tests & benchmarks.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for some simple rules.

I use Git Flow for this package so please use something similar or the usual GitHub workflow.

1. Fork it ( https://github.com/keltia/dmarc-rs/fork )
2. Checkout the develop branch (`git checkout develop`)
3. Create your feature branch (`git checkout -b my-new-feature`)
4. Commit your changes (`git commit -am 'Add some feature'`)
5. Push to the branch (`git push origin my-new-feature`)
6. Create a new Pull Request

[ASTERIX]: https://www.eurocontrol.int/asterix/
[Mozilla]: http://mozilla.org/
[RUST]: https://www.rust-lang.org/
[drone-gencsv: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey
[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html
[TOML]: https://github.com/naoina/toml/
