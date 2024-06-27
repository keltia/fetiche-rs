# Fetiche

## Pre-requesites

Fetiche is mainly made of [Rust] code with some Markdown documents for documentation, some [Python] scripts for
specific tasks and various administrative files linked to the Rust components (`Cargo.toml`, `README.md`, etc.).

You will need to have a stable channel Rust compiler and ecosystem (`cargo`, etc.). See the
[main site](https://www.rust-lang.org/tools/install) for instructions. Various Linux distributions have Rust in their
packages but most of them have rather old ones (i.e. Debian) so I'd recommend installed [rustup] first and use it to
fetch and install the toolchains (see above).

On macOS, `rustup` is available through [Homebrew] and this is the way I'd recommend to use:

```text
brew install rustup-init
```

For Windows, I've been using [Scoop] with great success to install, again, `rustup`:

```text
scoop install rustup
```

After `rustup` is installed, you need to install the toolchain(s) you need:

```text
rustup toolchain install stable
```

to get the latest version in the "stable" channel. To compile at least one of the optional components of fetiche, you
may need to install the "nightly" version which is update every few days. See [rustup] for more details.

After installing the toolchain, just verify you can access the main utilies like `rustc`, `cargo`, etc.:

```text
❯ cargo version
cargo 1.79.0 (ffa9cf99a 2024-06-03)
❯ rustc --version
rustc 1.79.0 (129f3b996 2024-06-10)
```

## Cloning the repository

Fetiche is hosted primarily on [Github] with clones on the different machines I work on.

If you are not connected already on your GitHub account or, you don't have one.

```text
git clone https://github.com/keltia/fetiche-rs
```

However, it is better to use the SSH-based client:

```text
git clone git@github.com:keltia/fetiche-rs
```

You can also fork the repository on your own GitHub account and clone it, it is better if you intend to submit patches.
You will also need to be able to fetching packages over the Internet through https so see with your system administrator
for eventual proxy setup, etc.

## Building the Rust applications

As explained in [FETICHE.md](FETICHE.md), fetiche is separated into several parts (called "crates" in Rust ecosystem)
and the various `Cargo.toml` files you can find in the tree specify the metadata linking all of this together.

### Libraries

Some of fetiche's crates are libraries and are not built by themselves. They are referenced through the dependency
sections in `Cargo.toml` and will be built for the binaries when you compile them.

### Binaries

As we are still using the "nightly" toolchain for the `opensky-history` binary, one can not just run `cargo build` from
the top directory, you will have to build both `acutectl` and `process-data` separately:

```text
cd acutectl && cargo build
cd process-data && cargo build
```

See [acutectl](../acutectl/README.md) and [process-data](../process-data/README.md) README for details. There is also
`opensky-history` but it is not used for now, we have another, internal, source of ADS-B data.

### Branches

The fetiche repository contains several important branches, some long-lived (`main`, `develop`) and some temporary ones
depending on the current development streams (`fetiche-ch`, `feature/whatever`, etc.). As of July 2024, `develop`
contains the branch compiling with the [DuckDB] embedded database whereas `fetiche-ch` is the branch for
the [Clickhouse] port. Fetiche is compliant with the [gitflow] process for editing and merging branches.

## Scripts

Fetiche contains several Python 3 scripts used to wrap the main utilities like `acutectl` for cron-based or manual
usages.

See [README.md](../scripts/README.md) for more details.

# References

[Clickhouse]: https://clickhouse.com/

[DuckDB]: https://duckdb.org/

[gitflow]: https://www.gitkraken.com/learn/git/git-flow

[GitHub]: https://github.com/

[Python]: https://python.net/

[Rust]: https://rust-lang.org/

[rustup]: https://rustup.rs/

[Scoop]: https://scoop.sh/
