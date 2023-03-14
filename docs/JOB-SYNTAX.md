## Purpose

We define a language to help specify where to fetch data from, what transformations if any we want to apply and then
export into a final format.

Th goal is to merge all the different iterations of `aeroscope.sh`, `aeroscope-cdg.sh` or `aeroscope-CDGweekly.sh` into
a more general purpose
data fetch & transform (aka a small-scale [ERP]).

## Configuration

Starting with a [HCL] configuration file like the one we have in the `sites` crate, we can either define a new one or
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
language, a symbol table, etc. In the latter case, like in `format-specs` right now, it is defined in the `output`
section.

Either we have a file per task, or we define everything in a single file and use naming to select one or the other.

If we take HCL, we could have something like this:

```hcl
task "weekly/asd" {
  site     = "asd"            // cf. sites/src input format is implied e.g. json or csv
  fetch    = "cmd.get"
  schedule = "Sun@01:00"  // ?? is it needed?  How do we schedule in practice?  cron is the ideal candidate
  into     = "cat21"          // cf. format-specs/output, maybe `output =  FORMAT`
  //...
}
```

## Schedule ?

cf. above.

## References

[HCL]: https://github.com/hashicorp/hcl/blob/main/hclsyntax/spec.md

[Vault]: https://crates.io/hashicorp_vault


