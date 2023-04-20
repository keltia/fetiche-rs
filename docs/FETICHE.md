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

[SURV Engine](file:SURV%20Engine.mmap) as a MindManager file.

### Modules

- formats
- sources
- outputs
- engine
- transformations
- filters
- CLI control interface to daemon/tasks
- API ?

### Sources Configuration

Starting with a [HCL] configuration file like the one we have in the `sources` crate, we can either define a new one or
extend this one.

Currently, we have something like this:

```hcl
sites "eih" {
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

sites "asd" {
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

## Task definition

The goal is to describe what needs to be done, where to fetch and possibly the transformations we need to apply to data
before converting. Or the transformations are implied by the task. If the former, we would have to define a more precise
language, a symbol table, etc. In the latter case, like in `format-specs`.

Either we have a file per task, or we define everything in a single file and use naming to select one or the other.

There are several possibilities to define tasks, filters and such.  Either we have our own language, but it could get complicated depending on how far we want to go, or we embed a language like Typescript or Lua.

### HCL

If we base ourselves to HCL and extend it, we could have something like this:

```hcl
task "weekly/asd" {
  site     = "asd"            // cf. sources/src input format is implied e.g. json or csv
  fetch    = "cmd.get"
  schedule = "Sun@01:00"  // ?? is it needed?  How do we schedule in practice?  cron is the ideal candidate
  into     = "cat21"          // cf. format-specs/output, maybe `output =  FORMAT`
  //...
}
```

### [Typescript]

[Deno] is a nice TS engine/runtime implemented in Rust.

````typescript
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
````

Typescript opens more opportunities to interact with the engine core. Filters, transformations could be written in
Typescript as well.  There are binding generators to tia the Rust API and use it in TS.

### Lua

[Lua] is also a viable extension language, already used by many projects (including [FreeBSD]).  The [mlua] crate does
provide access to Lua.  Main issues I see with Lua is incompatibility between the different versions.

## Schedule ?

cf. above.

## References

[HCL]: https://github.com/hashicorp/hcl/blob/main/hclsyntax/spec.md

[Vault]: https://crates.io/hashicorp_vault

[Typescript]: https://en.wikipedia.org/wiki/TypeScript 

[deno]: https://deno.land/

[deno-rs]: https://lib.rs/crates/deno_core

[mlua]: https://lib.rs/crates/mlua

[Lua]: https://www.lua.org/

[Kirikou]: https://en.wikipedia.org/wiki/Kirikou_and_the_Sorceress
