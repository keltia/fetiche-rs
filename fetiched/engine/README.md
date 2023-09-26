# **fetiche-engine**

## Engine

The engine is the main module. It offers a queueing system in which all tasks in the queue are interconnected
in a way that closely resemble a UNIX pipe. You typically have a producer, one or more filters and a
consumer at the end. Producers will generate or fetch data from a specific source and consumers are often
used for storing the data in different ways.

## Jobs

A job has an ID and a list of tasks which will be each executed by a different thread and all thread in
the pipe will be connected through channels. The Nth task's output will be a `Sender`  connected to the
`Receiver` on the next task.

## Tasks

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

## Producers

Producers are typically at the start of a job queue. They get or generate data in specific ways and send
them down the pipe. Best examples are `Fetch`  and `Stream`.

### Fetch

This used some API to fetch a file or chunk of data and send it down.

### Stream

This is used for streaming APIs, whether native like Flightaware or simulated ones (like we do with Opensky).

### Read

This is the same as `Fetch` but for a local file (think: reading a CSV file).

## Filters

Filters are only allowed between producers and consumers. Typically, you will use `Convert` when you need
to convert the upstream data into a different format and send it down the pipeline.

### Nothing, Message and Copy

These are there more to test and implement simple functions.

### Convert

At the moment, this task only support converting into our own `Cat21`  pseudo format, usually as CSV.

## Consumers

Consumers are used to store or duplicate data into different storage methods or even send data through
different means (imagine a Multicast task).

### Save

This task saves the data it received into a single file.

### Store

This task get all data from the upstream pipe and store it into a specific directory organized by Job ID
and using a different file for every hour.

### S3store (NOT IMPLEMENTED)

This is like the previous `Store`  but using an S3-compatible method.



