//! Preliminary results:
//!
//! Mac Studio 2020, M2 Pro, 64 GB RAM, 4 TB SSD
//! ```text
//!      Running benches/csv-to-parquet.rs (/Users/roberto/Src/Rust/src/fetiche-ch/target/release/deps/csv_to_parquet-db04090f49b98705)
//!
//! using_arrow2            time:   [179.19 ms 182.50 ms 186.09 ms]
//!
//! using_df                time:   [111.15 ms 113.77 ms 116.77 ms]
//! Found 1 outliers among 20 measurements (5.00%)
//!   1 (5.00%) high mild
//!
//! using_polars            time:   [68.143 ms 69.217 ms 70.438 ms]
//! Found 1 outliers among 20 measurements (5.00%)
//!   1 (5.00%) high mild
//! ```
//!
//! PC, Windows 10, Core i5 9600K, 12 GB, 500 MB SSD
//! ```text
//!
//!      Running benches\csv-to-parquet.rs (C:\Users\roberto\Src\rs\src\fetiche-rs\target\release\deps\csv_to_parquet-61edbcd77c3f9daa.exe)
//!
//! using_arrow2            time:   [357.84 ms 361.03 ms 364.42 ms]
//!                         change: [-13.136% -8.2239% -4.0771%] (p = 0.00 < 0.05)
//!
//! using_df                time:   [347.12 ms 352.41 ms 358.39 ms]
//!                         change: [-17.955% -10.217% -2.9559%] (p = 0.01 < 0.05)
//! Found 2 outliers among 20 measurements (10.00%)
//!   2 (10.00%) high mild
//!
//! using_polars            time:   [59.110 ms 60.930 ms 63.295 ms]
//!                         change: [-2.8290% +0.4680% +4.5345%] (p = 0.83 > 0.05)
//!
//! Found 3 outliers among 20 measurements (15.00%)
//!   1 (5.00%) high mild
//!   2 (10.00%) high severe
//! ```
//!
//! File sizes for 9999 records.
//! ```text
//! Mode                 LastWriteTime         Length Name
//! ----                 -------------         ------ ----
//! -a---          10/09/2024    18:49        3178176 test-bench.csv
//! -a---          21/10/2024    18:14         651711 test-arrow2.parquet
//! -a---          21/10/2024    18:14         667616 test-df.parquet
//! -a---          21/10/2024    18:15         603072 test-polars.parquet
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

use crate::arw::{read_csv, write_chunk};
use crate::df::parquet_through_df;
use crate::prs::parquet_through_polars;

fn use_arrow2(c: &mut Criterion) {
    let mut r = 1;

    eprintln!("start arrow2");
    c.bench_function("using_arrow2", |b| {
        b.iter(|| {
            let (s, d) = read_csv().unwrap();
            r = write_chunk(s, d, "test-arrow2").unwrap();
        })
    });
    let _ = r;
}

fn use_df(c: &mut Criterion) {
    eprintln!("start df");
    c.bench_function("using_df", |b| {
        b.to_async(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        )
        .iter(|| async { parquet_through_df().await.unwrap() });
    });
}

fn use_polars(c: &mut Criterion) {
    eprintln!("start polars");
    c.bench_function("using_polars", |b| {
        b.iter(|| black_box(parquet_through_polars().unwrap()))
    });
}

mod arw {
    use arrow2::{
        array::Array,
        chunk::Chunk,
        datatypes::Schema,
        io::csv::read::{
            deserialize_column, infer, infer_schema, read_rows, ByteRecord, ReaderBuilder,
        },
        io::parquet::write::{
            transverse, CompressionOptions, FileWriter, RowGroupIterator, Version, WriteOptions,
        },
    };
    use eyre::Result;
    use parquet2::{compression::ZstdLevel, encoding::Encoding};
    use rayon::prelude::*;
    use std::fs::File;

    const BATCH_SIZE: usize = 500000;

    pub fn read_csv() -> Result<(Schema, Vec<Chunk<Box<dyn Array>>>)> {
        let fname = "../data/test-bench.csv";

        let mut reader = ReaderBuilder::new().delimiter(b':').from_path(fname)?;
        let (fields, _) = infer_schema(&mut reader, None, true, &infer)?;
        let schema = Schema::from(fields.clone());

        // Read in batches of `BATCH_SIZE` elements.
        //
        let mut data = vec![];

        // Fill in with input data
        //
        loop {
            let mut rows = vec![ByteRecord::default(); BATCH_SIZE];
            let rows_read = read_rows(&mut reader, 0, &mut rows)?;

            // Are we finished?
            if rows_read == 0 {
                break;
            }
            let rows = &rows[..rows_read];

            // Now process all columns in parallel
            //
            let arrays: Vec<Box<dyn Array>> = fields
                .par_iter()
                .enumerate()
                .map(|(n, field)| deserialize_column(rows, n, field.data_type.clone(), 0).unwrap())
                .collect();

            let chunk = Chunk::new(arrays);

            data.push(chunk);
        }

        Ok((schema, data))
    }

    pub fn write_chunk(
        schema: Schema,
        data: Vec<Chunk<Box<dyn Array>>>,
        base: &str,
    ) -> Result<u32> {
        let options = WriteOptions {
            write_statistics: true,
            compression: CompressionOptions::Zstd(Some(ZstdLevel::try_new(8)?)),
            version: Version::V2,
            data_pagesize_limit: None,
        };

        // Prepare output
        //
        let fname = format!("../data/{}.parquet", base);
        let file = File::create(&fname)?;

        let iter: Vec<_> = data.iter().map(|e| Ok(e.clone())).collect();

        let encodings = schema
            .fields
            .iter()
            .map(|f| transverse(&f.data_type, |_| Encoding::Plain))
            .collect();

        let row_groups = RowGroupIterator::try_new(iter.into_iter(), &schema, options, encodings)?;
        let mut writer = FileWriter::try_new(file, schema, options)?;

        for group in row_groups {
            writer.write(group?)?;
        }

        let _ = writer.end(None).unwrap();
        Ok(0)
    }
}

mod df {
    use datafusion::config::TableParquetOptions;
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

        let mut options = TableParquetOptions::default();
        options.global.created_by = "bench_df".to_string();
        options.global.writer_version = "2.0".to_string();
        options.global.encoding = Some("plain".to_string());
        options.global.statistics_enabled = Some("page".to_string());
        options.global.compression = Some("zstd(8)".to_string());

        let _ = df.write_parquet(fname, dfopts, Some(options)).await?;

        Ok(())
    }
}

mod prs {
    use polars::prelude::*;

    pub fn parquet_through_polars() -> eyre::Result<()> {
        let fname = "../data/test-bench.csv";

        // nh = no header line (default = false which means has header line).
        //
        let header = true;
        let delim = b':';

        let mut df = CsvReadOptions::default()
            .with_has_header(header)
            .with_parse_options(CsvParseOptions::default().with_separator(delim))
            .try_into_reader_with_file_path(Some(fname.into()))?
            .finish()?;

        let fname = "../data/test-polars.parquet";

        let mut file = std::fs::File::create(fname)?;
        ParquetWriter::new(&mut file).finish(&mut df)?;
        Ok(())
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20).warm_up_time(Duration::from_secs(15));
    targets = use_arrow2, use_df, use_polars,
}

criterion_main!(benches);
