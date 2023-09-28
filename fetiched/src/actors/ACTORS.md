# This file describe the various actors and how they interact

- Config

This is a key-value store of all configuration.

- Engine

This is where most of the work is done, with a pipeline for each job.

- State

This actor is for registering and keeping state up to date. State is define by a tag or a name for the sub-system
and a JSON-encoded string representing the state of said sub-system. For `State` it is an opaque object, each actor
is supposed to encode/decode as needed.

- Storage

Here we define the various storage features like files, directories or even online storage like S3.
