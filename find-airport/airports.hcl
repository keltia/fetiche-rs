// Example configuration file for `find-airports`
//
version = 1

// Basedir for everything -- will load files into "{datalake}/files"
datalake = "/path/to/datalake"

// Source site
base_url = "https://davidmegginson.github.io/ourairports-data/"

// We do not need everything, these are csv, will be converted to parquet
sources = ["airports", "airport-frequencies", "navaids", "runways"]
