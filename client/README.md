# fetiche-client

This is the stub library part of the engine. It is the front end that all applications use.

This instantiates the `Engine` class directly.

> NOTE: the current API mirrors what `Engine` has, which is probably a bad idea.

# API Structure

We have a `Job` created, then have builders for the various types of jobs.

## Producers

- `fetch`

returns a `FetchBuilder`.

- `stream`

returns a `StreamBuilder`.



