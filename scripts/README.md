# Various scripts for different systems

## Shell (UNIX)

- `reduce.sh`

Takes CSV with full ADS-B data from EMIT and reduces them to pseudo-Cat21 format with specific columns.
Significant data is lost during the process. Uses the `adsb-hdr.txt` file to add CSV headers.

> NOTE: this is now obsolete as the fetiche workflow does not use or generate pseudo-Cat21 files.

- `fetch-fa.sh`

Automate the retrieval of ADS-B data from [FlightAware] [Firehose] API through `acutectl`.

## Python (both Windows and UNIX)

- `location-to-geojson.py`

Take the Lat/Lon coordinates from `locations.hcl` and generate a proper GeoJSON file for DuckDB.

- `fetch-opensky.py`

Python-only version of `opensky-history`, created to work around an incompatibility between the current "nightly" of
Rust and `inline-python`.

- `convert-csv.py`

This is the equivalent of `convert-ps1` but using Python and [bdt] instead of the soon-to-be obsolete `adsb-to-parquet`
because the former has more options and my pull request adding `-s` and `-z` has been merged.

- `dispatch-drops.py`

Take the parquet files coming from `convert-csv.py` and move them into the proper Hive subdirectory.

## Powershell (Windows)

- `convert-csv.ps1`

Automate the conversion of several CSV files into their Parquet equivalent on Windows.

> NOTE: this is now obsolete as the fetiche workflow does not use or generate pseudo-Cat21 files.

## References

[bdt]: https://github.com/datafusion-contrib/bdt

[Flightaware]: https://flightaware.com/

[Firehose]: https://www.flightaware.com/firehose/documentation/
