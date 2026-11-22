# opensky-history

This is the main CLI interface for retrieving the historical data from [Opensky].

## NOTE

This is a special app, it uses embedded Python through
It uses [SSH]  to connect to a specific shell on the `data.opensky-network.org` machine.
It supports only `pyopensky` v1.4 for now.  2.x API has completely changed.

## Building on Windows with Python 3.15 installed

PyO3 0.24 supports CPython through 3.13. Build this command with a side-by-side
64-bit Python 3.13 and Rust nightly, which is required by `inline-python`.
Use the same Python installation at build time and runtime because
`inline-python` compiles Python bytecode during the Rust build.

From the repository root in PowerShell, download and select the portable
[CPython 3.13.17 NuGet package](https://www.nuget.org/packages/python/3.13.17):

```powershell
$root = (New-Item -ItemType Directory -Force target/opensky-python).FullName
$pythonDir = Join-Path $root 'python-3.13.17/tools'
$pythonExe = Join-Path $pythonDir 'python.exe'
if (-not (Test-Path -LiteralPath $pythonExe)) {
    $archive = Join-Path $root 'python-3.13.17.zip'
    Invoke-WebRequest -Uri 'https://api.nuget.org/v3-flatcontainer/python/3.13.17/python.3.13.17.nupkg' -OutFile $archive
    Expand-Archive -LiteralPath $archive -DestinationPath (Join-Path $root 'python-3.13.17') -Force
}

$env:PYO3_PYTHON = $pythonExe
$env:PYTHONHOME = $pythonDir
$env:PATH = "$pythonDir;$env:PATH"

cargo +nightly test -p opensky-history --all-targets --locked
cargo +nightly run -p opensky-history --locked -- -V
```

If needed, install the Rust toolchain with `rustup toolchain install nightly`.
If you previously set `PYO3_USE_ABI3_FORWARD_COMPATIBILITY`, unset it for this
build. The limited ABI omits the marshal API used by `inline-python`.

The Python selection applies to the current PowerShell session. To make live
OpenSky requests, install the supported `pyopensky` version into this Python
environment using `& $env:PYO3_PYTHON -m pip` and configure your OpenSky access.

## USAGE

<details>
<summary>`opensky-history --help`</summary>

```text
Rust CLI app to fetch historical data through pyopensky.

Usage: opensky-history [OPTIONS] [START] [END]

Arguments:
  [START]  Start date (YYYY-MM-DD)
  [END]    End date (YYYY-MM-DD)

Options:
  -C, --config <CONFIG>  Location file path
  -n, --name <NAME>      Location name (if in `locations.hcl`)
  -o, --output <OUTPUT>  Output file (default is stdout)
  -R, --range <RANGE>    Detection range in nautical miles [default: 25]
  -h, --help             Print help
```

</details>

Give the command a location and use `-B` and `-E` to set the interval. A date such as
`2024-01-01` means midnight UTC. An ISO 8601 date-time can include an offset, as in
`2024-01-01T01:00:00+01:00`. A date-time without an offset uses the system time zone.

Due to the way the Opensky Impala is available, you have to specify the segment where the data you are interested in is,
otherwise queries will take a very long time. Right now, the app will figure it out for you given the date interval
you are giving it.

### Configuration

`opensky-history` use the [`inline-python`](https://crates.io/crates/inline-python) crate because it uses the `pyopensky` module as a wrapper for the 
SSH-based Impala access. So you will need Python 3, `pip` and `pyopensky` installed before  using the utility.

<details>
<summary>`secret.conf`</summary>

```ini
[default]
server = data.opensky-network.org
port = 2230
username = someone
password = GUESS
```

</details>

There is also an embedded location file in the app, called `locations.hcl`. It contains the various location where there
is an ADS-B receiver connected to the [Opensky]  network.

```hcl
version = 1

default = "Belfast"

location "Belfast" {
  lat = 54.7
  lon = -6.2
}

location "HQ" {
  lat = 50.8
  lon = 4.4
}

location "ILUX" {
  lat = 49.6
  lon = 6.2
}
```

[Opensky]: https://opensky.app/

[pyopensky]: https://github.com/open-aviation/pyopensky
