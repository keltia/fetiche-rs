# FETICHE

Preliminary name (based on the [Kirikou] movies made by Michel Ocelot, this is a small wooden fetish on top of the 
Witch's home looking all around and providing surveillance for her).

## Purpose

We define a program/framework/language to help specify where to fetch data from, what transformations if any we want
to apply and then export into a final format in various ways.

The goal is to merge all the different iterations of `aeroscope.sh`, `aeroscope-cdg.sh` or `aeroscope-CDGweekly.sh` into
a more general purpose data fetch & transform (aka a small-scale [ERP]).  It is not limited to drones and could be used as
a all-purpose gather/transform/publish engine for surveillance data.

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

### Sources Configuration

Starting with a [HCL] configuration file like the one we have in the `sources` crate, we can either define a new one or
extend this one.

Currently, we have something like this:

```hcl
version = 4

site "eih" {
  features = ["fetch"]
  format   = "aeroscope"
  base_url = "http://127.0.0.1:2400"
  auth     = {
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
  auth     = {
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
written in an embedded language.  This could be something homegrown with a specific grammar and a `nom`  parser 
(or equivalent).  This also could be a language easy to embed like Lua or Typescript.

Said language would have access to the various Rust data structures and methods which means generating bindings unless
we use our own system.

Possible example to follow could be [aurae] which use Typescript inside its own [auraescript].

## Choices

The goal is to describe what needs to be done, where to fetch and possibly the transformations we need to apply to data
before converting. Or the transformations are implied by the task. If the former, we would have to define a more precise
language, a symbol table, etc. In the latter case, like in `format-specs`.

Either we have a file per task, or we define everything in a single file and use naming to select one or the other.

There are several possibilities to define tasks, filters and such.  Either we have our own language, but it could get complicated depending on how far we want to go, or we embed a language like Typescript or Lua.

### Homegrown

### HCL

If we base ourselves to HCL and extend it, we could have something like this:

```hcl
task "weekly/asd" {
  site     = "asd"            // cf. sources/src input format is implied e.g. json or csv
  fetch    = "cmd.get"
  schedule = "Sun@01:00"  // ?? is it needed?  How do we schedule in practice?  cron is the ideal candidate
  into     = "cat21"          // cf. formats/output, maybe `output =  FORMAT`
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
Typescript as well.  There are binding generators to tia the Rust API and use it in TS.

NOTE: It is yet not clear where the boundaries of Rust and Typescript lie.  Even the source crate or the format-specs one
could be seen in [TS] as well.  Having a proper definition of what is available to [TS] is essential.

### Lua

[Lua] is also a viable extension language, already used by many projects (including [FreeBSD]).  The [mlua] crate does
provide access to Lua.  Main issues I see with Lua is incompatibility between the different versions.

I'm not sure how much of a support we have to access Rust data from Lua.  Main advantage over [TS]  would be 
the size of the implementation although Rust binaries tend to be rather big anyway.

## Schedule ?

cf. above.

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
