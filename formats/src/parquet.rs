//! Parquet (https://parquet.apache.org/docs/file-format/) support as an output file format
//!
//! Every `struct`  that support Parquet output must be marked with a `ParquetRecordWriter` derive
//! tag and needs to be flat (no inside struct, etc.).
//!

use std::io::Write;

use eyre::Result;
use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::schema::types::TypePtr;
use parquet::{
    file::{properties::WriterProperties, writer::SerializedFileWriter},
    record::RecordWriter,
};
use tracing::trace;

use crate::version;

#[tracing::instrument(skip(data, out))]
pub fn into_parquet<T, U>(data: &[U], out: &mut (dyn Write + Send)) -> Result<()>
where
    U: RecordWriter<T>,
{
    trace!("{} records", data.len());
    let schema: TypePtr = data[0].schema()?;

    let props = WriterProperties::builder()
        .set_created_by(version())
        .set_encoding(Encoding::PLAIN)
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    let mut writer = SerializedFileWriter::new(out, schema, props.into())?;
    let mut row_group = writer.next_row_group()?;

    trace!("Writing data.");
    data.iter()
        .for_each(|line| line.write_to_row_group(&mut row_group).unwrap());
    trace!("Done.");

    Ok(())
}
