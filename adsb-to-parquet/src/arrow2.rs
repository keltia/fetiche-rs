//! Module using `arrow2` to read and write parquet files.
//!
//! This is slightly faster than `datafusion`  for small datasets and generates parquet v2 files.
//!

use std::fs::File;
use std::time::Instant;

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
use tracing::{debug, info, trace};

use crate::Options;

/// Arbitrary value to get big row groups but not too big
///
const BATCH_SIZE: usize = 500000;

/// Read a csv file through batches of `BATCH_SIZE` lines
///
#[allow(clippy::type_complexity)]
#[tracing::instrument]
pub fn read_csv(fname: &str, opt: Options) -> Result<(Schema, Vec<Chunk<Box<dyn Array>>>)> {
    trace!("Read data.");

    trace!("fname={:?}", fname);

    let mut reader = ReaderBuilder::new().delimiter(opt.delim).from_path(fname)?;
    let (fields, _) = infer_schema(&mut reader, None, opt.header, &infer)?;
    let schema = Schema::from(fields.clone());

    // Read in batches of `BATCH_SIZE` elements.
    //
    let mut total = 0;
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

        // Count by lines, not bytes.
        //
        let size = rows.len();

        total += size;

        // Now process all columns in parallel
        //
        let arrays: Vec<Box<dyn Array>> = fields
            .par_iter()
            .enumerate()
            .map(|(n, field)| deserialize_column(rows, n, field.data_type.clone(), 0).unwrap())
            .collect();

        let chunk = Chunk::new(arrays);
        debug!("arrays={:?}", chunk);

        data.push(chunk);
    }
    info!("{} lines in {} batches.", total, data.len());

    Ok((schema, data))
}

/// Write a parquet file from a vector of datasets.  Each dataset will end up in a dedicated row
/// group.
///
/// Use parquet v2 format, ZSTD compression at level 8
///
#[tracing::instrument(skip(schema, data))]
pub fn write_chunk(schema: Schema, data: Vec<Chunk<Box<dyn Array>>>, fname: &str) -> Result<u64> {
    let options = WriteOptions {
        write_statistics: true,
        compression: CompressionOptions::Zstd(Some(ZstdLevel::try_new(8)?)),
        version: Version::V2,
        data_pagesize_limit: None,
    };

    debug!("data in={:?}", data);

    // Prepare output
    //
    let file = File::create(fname)?;

    let start = Instant::now();
    let iter: Vec<_> = data.iter().map(|e| Ok(e.clone())).collect();
    debug!("iter={:?}", iter);

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

    let size = writer.end(None)?;
    info!("{} bytes written.", size);

    let tm = Instant::now() - start;
    Ok(tm.as_millis() as u64)
}
