## VERSION INFORMATION

This is release v0.50.99 (next version TBD)

It has many fundamental changes under the hood:

- [DuckDB] has been replaced by [Clickhouse] for all calculations.
- `process-data` is now completely *async* due to many dependencies including the `klickhouse` client.
- `acutectl` & `process-data` are now *async*.
- There are many more Python scripts in the `scripts` directory.
- Most of the import and conversion processes make use of the [bdt] and [qsv] utilities.
- more actors are implemented within `fetiche-sources`.
- `fetiche-sources` has been merged as integral part of the engine.
- `fetiched` is currently not working and will be updated in the near-future.

Main dependencies:

- polars 0.46 (it has replaced datafusion & arrow/parquet)

Current crates versions:

- acutectl/0.23.1
- process-data/0.7.0+clickhouse
- fetiche-engine/0.24.0
- fetiche-formats/0.18.0
- fetiche-common/0.4.0
- fetiche-macros/0.3.0
