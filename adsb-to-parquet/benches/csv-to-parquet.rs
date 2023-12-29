use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

use crate::arw::{read_csv, write_chunk};
use crate::df::parquet_through_df;

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
    use datafusion::dataframe::DataFrameWriteOptions;
    use datafusion::parquet::basic::{Compression, Encoding, ZstdLevel};
    use datafusion::parquet::file::properties::{EnabledStatistics, WriterProperties};
    use datafusion::prelude::*;
    use eyre::Result;

    pub async fn parquet_through_df() -> Result<()> {
        let fname = "../data/test-bench.csv";

        // nh = no header line (default = false which means has header line).
        //
        let header = true;
        let delim = b':';

        let ctx = SessionContext::new();
        let copts = CsvReadOptions::new().delimiter(delim).has_header(header);

        let df = ctx.read_csv(fname, copts).await?;

        let fname = "../data/test-df.parquet";

        let dopts = DataFrameWriteOptions::default().with_single_file_output(true);
        let props = WriterProperties::builder()
            .set_created_by("bench_df".to_string())
            .set_encoding(Encoding::PLAIN)
            .set_statistics_enabled(EnabledStatistics::Page)
            .set_compression(Compression::ZSTD(ZstdLevel::try_new(8)?))
            .build();

        let _ = df.write_parquet(&fname, dopts, Some(props)).await?;

        Ok(())
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20).warm_up_time(Duration::from_secs(10));
    targets = use_arrow2, use_df
}

criterion_main!(benches);
