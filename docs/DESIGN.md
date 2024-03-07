# Description of the curent design of Fetiche and future plans

Updated: Sun Feb 25 19:41:21 CET 2024

## Current Design

`Fetiche` as a framework was mainly born as a refactor of the code used by the `cat21conv` utility whci is the rewrite
in [RUST] of the original Shell script. Code to handle formats and sites (aka sources) were moved into libraries.

As the ACUTE project evolved, needs did as well and more code was added to handle these.

- `fetiche-common` has now some of the common code used by all crates.
- `fetiche-formats` is handling the various data structures used throughout the framework, dealing with conversion,
  serialisation and de-serialisation.
- `fetiche-sources` contains the code to connect to various sites and fetch or stream data out of them. It also handles
  authentication, etc.
- `fetiche-macros` is the specific crate hosting the `RunnableDerive` proc macro for the engines.

There are also several binaries using the framework:

- `cat21conv` is the original script, very tied to the cut-down Cat21-like format which is a CSV-compatible version of
  the original Cat21 [ASTERIX] binary format.
- `acutectl` replaces it with better interface, more format support (like JSON, Parquet).
- `opensky-history` was created to fetch historical data out of the [OPENSKY] site for ADS-B data. It uses the Python
  package called `pyopensky` through the `inline-python` crate. There is a pure-Python version in the scripts directory.
- `process-data` is related to `acutectl` as the main user of the data fetched by it. It interacts with a [DuckDB]
  database which handle all the data for [ACUTE].

`fetiched` is a bit special, it is supposed to be a daemon handling all aspects of data fetching and transformation, and
it seems that it might not be the best approach for [ACUTE]. It has a fork of the main engine which resides
inside `acutectl` and they will have to be reunited at some point. It lags behind the other engine at the moment.

## Future Evolution

Right now, having two crates for formats and sources does make modifications a bit complicated as these two have to be
in sync all the time.

One possible evolution would be to have some kind of plugin for every source. That plugin would handle both data formats
and data retrieval through some traits.

- ASD with the data formats, connection to site, etc.
- OpenSky with data format, streaming and historical data, etc.

And `acutectl`  could have crate features to include some of the plugins or not.

### Traits

- Fetchable
- Streamable



