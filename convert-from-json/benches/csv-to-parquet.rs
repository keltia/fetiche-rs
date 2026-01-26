//! Preliminary results:
//!
//! Mac Studio 2020, M2 Pro, 64 GB RAM, 4 TB NVMe SSD, macOS 26.1 Tahoe
//! ```text
//!
//! using_df                time:   [58.125 ms 58.316 ms 58.516 ms]
//!
//! using_polars            time:   [38.088 ms 38.203 ms 38.352 ms]
//! ```
//!
//! PC, Windows 11 24H2, AMD 7700X, 32 GB, 500 MB M2 SSD
//! ```text
//!
//! using_df                time:   [165.94 ms 166.68 ms 167.40 ms]
//!
//! using_polars            time:   [80.382 ms 81.159 ms 81.979 ms]
//! ```
//!
//! File sizes for 9999 records.
//! ```text
//! Mode                 LastWriteTime         Length Name
//! ----                 -------------         ------ ----
//! -a---          21/05/2025    16:10        3178176 test-bench.csv
//! -a---          24/11/2025    22:36         674664 test-df.parquet
//! -a---          24/11/2025    22:37         569587 test-polars.parquet
//! ```
//!
//! 11/2025 UPDATE: datafusion is getting better speed-wise, but almost twice as slow as polars and
//! file size remains bigger.

use std::hint::black_box;
use std::time::Duration;

use crate::df::parquet_through_df;
use crate::prs::parquet_through_polars;

use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

fn use_df(c: &mut Criterion) {
    c.bench_function("using_df", |b| {
        b.to_async(Runtime::new().unwrap())
            .iter(|| async { parquet_through_df().await.unwrap() });
    });
}

fn use_polars(c: &mut Criterion) {
    c.bench_function("using_polars", |b| {
        b.iter(|| black_box(parquet_through_polars().unwrap()))
    });
}

mod df {
    use datafusion::common::parquet_config::DFParquetWriterVersion;
    use datafusion::config::{ParquetOptions, TableParquetOptions};
    use datafusion::dataframe::DataFrameWriteOptions;
    use datafusion::prelude::*;
    use eyre::Result;

    pub async fn parquet_through_df() -> Result<()> {
        let fname = "../data/test-bench.csv";

        // nh = no header line (default = false which means has header line).
        //
        let header = true;
        let delim = b':';

        let ctx = SessionContext::new();
        let df = ctx
            .read_csv(
                fname,
                CsvReadOptions::default()
                    .delimiter(delim)
                    .has_header(header),
            )
            .await?;

        let fname = "../data/test-df.parquet";

        let dfopts = DataFrameWriteOptions::default().with_single_file_output(true);
        let mut wpopts = ParquetOptions::default();
        wpopts.writer_version = DFParquetWriterVersion::V2_0;
        wpopts.compression = Some("zstd(8)".into());
        wpopts.encoding = Some("plain".into());
        wpopts.created_by = "bench_df".to_string();
        wpopts.statistics_enabled = Some("page".into());

        let mut opts = TableParquetOptions::new();
        opts.global = wpopts;
        let _ = df.write_parquet(fname, dfopts, Some(opts)).await?;

        Ok(())
    }
}

mod prs {
    use polars_io::prelude::*;

    pub fn parquet_through_polars() -> eyre::Result<()> {
        let fname = "../data/test-bench.csv";

        // nh = no header line (default = false which means has header line).
        //
        let mut df = CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(fname.into()))?
            .finish()?;

        let fname = "../data/test-polars.parquet";

        let mut file = std::fs::File::create(fname)?;
        ParquetWriter::new(&mut file)
            .with_compression(ParquetCompression::Zstd(Some(ZstdLevel::try_new(8)?)))
            .finish(&mut df)?;
        Ok(())
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20).warm_up_time(Duration::from_secs(15));
    targets = use_df, use_polars,
}

criterion_main!(benches);
