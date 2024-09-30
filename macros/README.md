# README.md

This crate add some [proc macros] to the `fetiche` framework

## RunnableDerive

This is a *derive* proc macro that implement the `Runnable` trait on aa given struct to allow it to be "executed" later
through the engine's Task system.

## add_version

This *attribute* macro add a `version` field to any struct and a specific version can be specified as well.

References:

[proc macros]: https://doc.rust-lang.org/reference/procedural-macros.html
