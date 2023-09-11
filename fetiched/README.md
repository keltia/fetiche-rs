
# fetiched

This is the daemon component of the Fetiche framework. It will be using the various components of `Fetiche` like the
engine and sources to implement a client-server approach.

It is not supposed to be called directly but it should be triggered by any call to `acutectl` or similar CLI utility
when it sees the daemon is not running. Or you can run `fetiched -D server`  in debug mode where all messages are sent
to stderr.

## Design

I decided to try to use [Actix] and the [Actor] pattern to get familiar with it, it seems interesting. On UNIX, unless
you
use debug mode, it can detach itself from the terminal with `stdout` and `stderr` redirection. PID is also managed
directly through the `.pid_file(pid)` call to `Daemonize::new()`.

The main actors right now are the `Config` one which is supposed to hold the configuration for the whole framework and
`Engine`, as it name implies, instantiate the `Engine` component to manage jobs, etc.

NOTE: the [Actix] run-time is full-async, using [Tokyo] as its main async engine.  `fetiche-engine` and its sub-module
`fetiche-sources` are full sync and concurrency is not yet an issue so the actors "wrap" the sync components.

## USAGE

<details>
<summary>`fetiched help`</summary>

```text
Daemon component of Fetiche.

Usage: fetiched.exe [OPTIONS] <COMMAND>

Commands:
  config    Display current config
  server    Run as a daemon (mostly for Windows)
  shutdown  Shutdown everything
  status    Daemon status
  version   List all package versions
  help      Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  configuration file
  -D, --debug            debug mode (no fork & detach)
  -v, --verbose...       Verbose mode
  -h, --help             Print help
```

</details>


[Actix]: https://actix.rs/

[Actor]: https://en.wikipedia.org/wiki/Actor_model

[Tokyo]: https://crates.io/crates/tokyo
