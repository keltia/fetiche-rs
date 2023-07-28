# opensky-history

This is the main CLI interface for retrieving the historical data from [Opensky].

## NOTE

This is a special app, it uses embedded Python through
It uses [SSH]  to connect to a specific shell on the `data.opensky-network.org` machine.

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

You MUST give it a location (see below). If no dates are specified, it will retrieve data for the current day.

Due to the way the Opensky Impala is available, you have to specify the segment where the data you are interested in is,
otherwise queries will take a very long time. Right now, the app will figure it out for you given the date interval
you are giving it.

### Configuration

`opensky-history` use the [`inline-python`](https://crates.io/crates/inline-python) crate because it uses
the `pyopensky`
module as a wrapper for the SSH-based Impala access. So you will need Python 3, `pip` and `pyopensky` installed before
using the utility.

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

