<!-- omit in TOC -->

# fetch-sac

> **Library to fetch the latest list of SAC codes from the official ECTL website**

[![Build status](https://github.com/keltia/fetch-sac/actions/workflows/rust.yml/badge.svg)](https://github.com/keltia/fetch-sac/actions/workflows/rust.yml)
[![Buildstatus (develop)](https://github.com/keltia/fetch-sac/actions/workflows/develop.yml/badge.svg)](https://github.com/keltia/fetch-sac/actions/workflows/develop.yml)
[![Docs](https://img.shields.io/docsrs/dmarc-rs)](https://docs.rs/drone-utils)
[![GitHub release](https://img.shields.io/github/release/keltia/dmarc-rs.svg)](https://github.com/keltia/fetch-sac/releases/)
[![GitHub issues](https://img.shields.io/github/issues/keltia/fetch-sac.svg)](https://github.com/keltia/fetch-sac/issues)
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

This is a small CLI utility to fetch the official list of [SAC codes] from the [ECTL] [Asterix] website.

## History

[ECTL] is the official maintainer of the worldwide list of [SAC codes], representing different zones in the world.  
These are used in Surveillance work in the Aeronautical world to represent a given (and large) zone from which a given
surveillance record has been issued when using the [Asterix] specifications.

This thing is, this list of **not** available in any usable format and you are supposed to just read the web page. This
is for me clearly unacceptable in 2023 and getting the list in various formats like [JSON] or even [CSV]  is desirable.

## Installation

It will be available at some point as crates on [Crates.io]  but for the moment just as a repository on
[GitHub]. Installation can be done either through a compiled binary for your platform or by cloning the repo and
compiling.

## Usage

For the moment, there is only one binary called `fetchsac` (with `.exe` on Windows). It scrapes the official website,
remove all the HTML and outputs the result into usable formats.

## MSRV

The Minimum Supported Rust Version is *1.56* due to the 2021 Edition.

## Supported platforms

* Unix (tested on FreeBSD, Linux and macOS)
* Windows
  * cmd.exe
  * Powershell

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for some simple rules.

I use Git Flow for this package so please use something similar or the usual GitHub workflow.

1. Fork it [here](https://github.com/keltia/fetch-sac/fork)
2. Checkout the develop branch (`git checkout develop`)
3. Create your feature branch (`git checkout -b my-new-feature`)
4. Commit your changes (`git commit -am 'Add some feature'`)
5. Push to the branch (`git push origin my-new-feature`)
6. Create a new Pull Request

[Asterix]: https://www.eurocontrol.int/asterix/

[RUST]: https://www.rust-lang.org/

[fetchsac: 1.56+]: https://img.shields.io/badge/Rust%20version-1.56%2B-lightgrey

[Rust 1.56]: https://blog.rust-lang.org/2021/10/21/Rust-1.56.0.html

[ECTL]: https://www/eurocontrol.int/
