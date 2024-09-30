## VERSION INFORMATION

This is release v0.50.

It has many fundamental changes under the hood:

- [DuckDB] has been replaced by [Clickhouse] for all calculations.
- `process-data` is now completely *async* due to many dependencies including the `klickhouse` client.
- `acutectl` is now *async* except for the `Engine` itself which still runs in a sync environment.
- There are many more Python scripts in the `scripts` directory.
- Most of the import and conversion processes make use of the [bdt] and [qsv] utilities.
- `fetiched` is currently not working and will be updated in the near-future.

Main dependencies:

- datafusion v41
- arrow/parquet v52

Current crates versions:

- acutectl/0.23.0
- process-data/0.4.0+clickhouse
- fetiche-engine/0.23.0
- fetiche-formats/0.17.1
- fetiche-sources/0.16.0
- fetiche-common/0.4.0
- fetiche-macros/0.3.0
