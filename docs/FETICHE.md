# FETICHE

## Purpose

We define a program/framework/language to help specify where to fetch data from, what transformations if any we want
to apply and then export into a final format in various ways.

The goal is to merge all the different iterations of `aeroscope.sh`, `aeroscope-cdg.sh` or `aeroscope-CDGweekly.sh` into
a more general purpose data fetch & transform (aka a small-scale [ERP]). It is not limited to drones and could be used
as an all-purpose gather/transform/publish engine for surveillance data.

# Description of the current design of Fetiche and future plans

Updated: Sun Feb 25 19:41:21 CET 2024

## Current Design

`Fetiche` as a framework was mainly born as a refactor of the code used by the `cat21conv` utility whci is the rewrite
in [RUST] of the original Shell script. Code to handle formats and sites (aka sources) were moved into libraries.

As the ACUTE project evolved, needs did as well and more code was added to handle these.

`Fetiche` has 4 main library component so far:

- `fetiche-common` has now some of the common code used by all crates.
- `fetiche-engine` is the main fetch/stream component, using the formats and sources crates.
- `fetiche-formats` is handling the various data structures used throughout the framework, dealing with conversion,
  serialisation and de-serialisation.
- `fetiche-sources` contains the code to connect to various sites and fetch or stream data out of them. It also handles
  authentication, etc.
- `fetiche-macros` is the specific crate hosting useful macros such as the `RunnableDerive` proc macro for the engines.

### Formats (managed in the `fetiche-formats` crate)

This crate implement the various data models used by the different sources. Included are three [ASTERIX]-like formats --
generic `Cat21`, a cut-down version of `Cat21` for ADS-B data (dubbed `Adsb21`) and drone-specific `Cat129` -- and
formats used by different data providers like [Opensky] or [ASD]. This library implement some methods of conversion
between some of these formats.

The default input format is the one used by the Aeroscope from ASD, but it will soon support the format used
by [Opensky] site. There is also the [ASD] site which gives you data aggregated from different Aeroscope antennas.

More details in the [Formats README.md](formats/README.md).

### Sources (managed in the `fetiche-sources` crate)

The configuration for the different sources of data is handled by the `fetiche-source` crate in [HCL] file
format. Note that it is mainly used to avoid hard-coding some parameters like username and API URLs. Adding an entry
in that file does not mean support except if it is a variation on a known source.

You are not really supposed to edit this file.

More details in the specific [Sources README.md](sources/README.md).

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
- AsyncStreamable

## Structure

Preliminary list of possible modules.

[SURV Engine](file:Fetiche%20Engine.mmap) as a MindManager file.

### Modules

- engine
- formats
- sources
- outputs
- transformations
- filters
- CLI control interface to daemon/tasks
- API ?

The current design has its pros and cons. It has the disadvantage of requiring modifying several crates when something
new like a site is introduced. We need to add code in `sources`  to implement operations like `fetch`, `formats`  to
implement encoding/decoding and conversion, etc. and teach `acutectl` and possibly others about these.

We may go for "all batteries included" crates where each one implement the various traits (let's call it module or
plugin) but that's longer term work.

### Sources Configuration

Starting with an [HCL] configuration file like the one we have in the `sources` crate, we can either define a new one or
extend this one.

Currently, we have something like this:

```hcl
version = 4

site "eih" {
  features = ["fetch"]
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth = {
    login    = "SOMETHING"
    password = "NOPE"
    token    = "/login"
  }
  cmd = {
    get = "/drone/get"
  }
}

site "asd" {
  features = ["fetch"]
  format   = "asd"
  base_url = "https://eur.airspacedrone.com"
  auth = {
    login    = "USERNAME"
    password = "GUESS"
    token    = "/api/security"
  }
  cmd = {
    get = "/api/journeys/filteredlocations"
  }
}
```

We could use the [`hashicorp_vault`][Vault] crate to fetch credentials though.

With this we obtain a `HashMap` with the site name as key (e.g. `eih`) and the rest as a `Site` struct.

# Extension Language

To minimize the amount of modifications necessary during the life of the product, it is intended for many parts to be
written in an embedded language. This could be something homegrown with a specific grammar and a `nom`  parser
(or equivalent). This also could be a language easy to embed like Lua or Typescript.

Said language would have access to the various Rust data structures and methods which means generating bindings unless
we use our own system.

Possible example to follow could be [aurae] which use Typescript inside its own [auraescript].

## Choices

The goal is to describe what needs to be done, where to fetch and possibly the transformations we need to apply to data
before converting. Or the transformations are implied by the task. If the former, we would have to define a more precise
language, a symbol table, etc. In the latter case, like in `format-specs`.

Either we have a file per task, or we define everything in a single file and use naming to select one or the other.

There are several possibilities to define tasks, filters and such. Either we have our own language, but it could get
complicated depending on how far we want to go, or we embed a language like Typescript or Lua.

### Homegrown

### HCL

If we base ourselves to HCL and extend it, we could have something like this:

```hcl
task "weekly/asd" {
  site = "asd"            // cf. sources/src input format is implied e.g. json or csv
  fetch = "cmd.get"
  schedule = "Sun@01:00"  // ?? is it needed?  How do we schedule in practice?  cron is the ideal candidate
  into  = "cat21"          // cf. formats/output, maybe `output =  FORMAT`
  //...
}
```

### [Typescript]

[Deno] is a nice TS engine/runtime implemented in Rust.

```typescript
// @ts-ignore
import {Task, TaskRequest, SchedulerClient} from "../lib/runtime.ts";

let sched = new SchedulerClient();

sched.new(<TaskRequest>{
    task: Task.fromPartial({
        name: "test",
    })
}).then(r => {
    console.log("done")
});
```

Typescript opens more opportunities to interact with the engine core. Filters, transformations could be written in
Typescript as well. There are binding generators to tia the Rust API and use it in TS.

NOTE: It is yet not clear where the boundaries of Rust and Typescript lie. Even the source crate or the format-specs one
could be seen in [TS] as well. Having a proper definition of what is available to [TS] is essential.

### Lua

[Lua] is also a viable extension language, already used by many projects (including [FreeBSD]). The [mlua] crate does
provide access to Lua. Main issues I see with Lua is incompatibility between the different versions.

I'm not sure how much of a support we have to access Rust data from Lua. Main advantage over [TS]  would be
the size of the implementation although Rust binaries tend to be rather big anyway.

## Schedule ?

cf. above.

## Why the name Fetiche?

Preliminary name (based on the [Kirikou] movies made by Michel Ocelot. If you have seen the animated movie "Kirikou et
la sorcière", you know. If not, learn that the main characters are Kirikou, a small boy with extraordinary gifts and
a sorceress called Karaba. She has some special minions called "les fétiches". These are wooden fetishes that do her
bidding in all things. One of them is on top of her case and is tasked with looking around and doing... surveillance.
Hence, the name of its framework.

## References

[HCL]: https://github.com/hashicorp/hcl/blob/main/hclsyntax/spec.md

[Vault]: https://crates.io/hashicorp_vault

[TS]: https://en.wikipedia.org/wiki/TypeScript

[deno]: https://deno.land/

[deno-rs]: https://lib.rs/crates/deno_core

[mlua]: https://lib.rs/crates/mlua

[Lua]: https://www.lua.org/

[Kirikou]: https://en.wikipedia.org/wiki/Kirikou_and_the_Sorceress

[aurae]: https://github.com/aurae-runtime/aurae

[auraescript]: https://github.com/aurae-runtime/aurae/tree/main/auraescript
