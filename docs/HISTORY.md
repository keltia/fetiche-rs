<!-- omit in TOC -->

# HISTORY.md

1. [History](#history)
2. [Configuration](#configuration-history)

## History

For the **ACUTE** Project, Marc Gravis wrote the Shell script `aeroscope.sh` in 2021 to fetch data from the aeroscope
server in EIH and transform it into a pseudo-Cat21 CSV file using the same field as the Category 21 from [ASTERIX]
Specifications. It uses `wget(1)` to fetch data and `jq(1)` and `awk(1)`  to transform it.

It works fine, but it is a bit fragile, has some hardcoded paths & filenames. This is an attempt at rewriting it
in [RUST], a fast and safe language defined in 2010 by [Mozilla]. It has been since evolved into a set of libraries and
binaries.

It is now known as the **Fetiche** surveillance framework.

## Configuration History

The `sources.hcl` configuration file is versioned to avoid incompatibilities.

- v1 was the original version with the `Sites` struct
- In v2 `Sites`  was renamed into `Sources` to reflect evolution
- In v3 the `type`  keyword was added to the `Site` definition
- In v4 the `features` keyword was added to indicate what is supported between `fetch` and `stream`.

The `formats.hcl` add metadata about all supported formats.

- v1 was generic
- v2 added the datatype for each format

The `dbfile.hcl` list possible database connections.

- v1 as the original design, to be evolved.

