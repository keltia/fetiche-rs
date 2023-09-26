# fetiched

This is the daemon component of the Fetiche framework. It will be using the various components of `Fetiche` like the
engine and sources to implement a client-server approach.

It is not supposed to be called directly, but it should be triggered by any call to `acutectl` or similar CLI utility
when it sees the daemon is not running. Or you can run `fetiched -D server`  in debug mode where all messages are sent
to stderr.

## Design

I decided to try to use [Actix] and the [Actor] pattern to get familiar with it, it seems interesting. On UNIX, unless
you use debug mode, it can detach itself from the terminal with `stdout` and `stderr` redirection. PID is also managed
directly through the `.pid_file(pid)` call to `Daemonize::new()`. On Windows, detaching is not available so the binary
will just run and stay attached to the current terminal.

The main actors right now are the `Config` one which is supposed to hold the configuration for the whole framework and
`Engine`, as it name implies, instantiate the `Engine` component to manage jobs, etc.

NOTE: the [Actix] run-time is full-async, using [Tokyo] as its main async engine.  `fetiche-engine` and its submodule
`fetiche-sources` are full sync and concurrency is not yet an issue so the actors "wrap" the sync components.

## **fetiche-engine**

### Engine

The engine is the main module. It offers a queueing system in which all tasks in the queue are interconnected
in a way that closely resemble a UNIX pipe. You typically have a producer, one or more filters and a
consumer at the end. Producers will generate or fetch data from a specific source and consumers are often
used for storing the data in different ways.

### Jobs

A job has an ID and a list of tasks which will be each executed by a different thread and all thread in
the pipe will be connected through channels. The Nth task's output will be a `Sender`  connected to the
`Receiver` on the next task.

### Tasks

Each task is defined with a struct which has the `Runnable Derive` derive pragma defined. This corresponds
to a proc-macro that will generate two methods in the trait: `cap()` to get the type of task (used in the
job runner to check that the pipe is valid)  and `run()`  which is the main thread executing the job.

For this, each task MUST define an `execute()`  method that will be called for each packet received
by the `run()` thread.

The current tasks defined are:

- `Nothing`
- `Message`
- `Copy`
- `Convert`
- `Fetch`
- `Read`
- `Save`
- `Store`
- `S3store`
- `Stream`

I think it is more flexible to work within the framework of the engine.

### Producers

Producers are typically at the start of a job queue. They get or generate data in specific ways and send
them down the pipe. Best examples are `Fetch`  and `Stream`.

#### Fetch

This used some API to fetch a file or chunk of data and send it down.

#### Stream

This is used for streaming APIs, whether native like Flightaware or simulated ones (like we do with Opensky).

#### Read

This is the same as `Fetch` but for a local file (think: reading a CSV file).

### Filters

Filters are only allowed between producers and consumers. Typically, you will use `Convert` when you need
to convert the upstream data into a different format and send it down the pipeline.

#### Nothing, Message and Copy

These are there more to test and implement simple functions.

#### Convert

At the moment, this task only support converting into our own `Cat21`  pseudo format, usually as CSV.

### Consumers

Consumers are used to store or duplicate data into different storage methods or even send data through
different means (imagine a Multicast task).

#### Save

This task saves the data it received into a single file.

#### Store

This task get all data from the upstream pipe and store it into a specific directory organized by Job ID
and using a different file for every hour.

#### S3store (NOT IMPLEMENTED)

This is like the previous `Store`  but using an S3-compatible method.

## USAGE

<details>
<summary>`fetiched help`</summary>

```text
Daemon component of Fetiche.

Usage: fetiched [OPTIONS] <COMMAND>

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
