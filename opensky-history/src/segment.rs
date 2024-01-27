use arrow2::array::Array;
use arrow2::chunk::Chunk;
use arrow2::io::csv::read;
use arrow2::io::csv::read::deserialize_batch;
use csv::ReaderBuilder;
use seek_bufread::BufReader;
use tracing::trace;

const ONE_HOUR: i32 = 3_600;

/// Read the csv segment
///
pub fn read_segment(seg: &str) -> eyre::Result<Chunk<Box<dyn Array>>> {
    let buf = BufReader::new(seg.as_bytes());
    let mut reader = ReaderBuilder::new().from_reader(buf);
    let (fields, _) = read::infer_schema(&mut reader, None, true, &read::infer)?;
    let mut rows = vec![read::ByteRecord::default(); 100];
    let rows_read = read::read_rows(&mut reader, 0, &mut rows)?;
    let rows = &rows[..rows_read];
    let r = deserialize_batch(rows, &fields, None, 0, read::deserialize_column)?;
    Ok(r)
}

/// Calculate the list of 1h segments necessary for a given time interval
///
/// Algorithm for finding which segments are interesting otherwise Impala takes forever to
/// retrieve data
///
/// All timestamps are UNIX-epoch kind of timestamp.
///
/// start = NNNNNN
/// stop = MMMMMM
///
/// i(0) => beg_hour = NNNNNN
/// i(N) => end_hour = MMMMMM - (MMMMMM mod ONE_HOUR)
///
/// N =  (MMMMMM - NNNNNN) / ONE_HOUR
///
/// thus
///
/// [beg_hour <= start] ... [end_hour <= stop]
/// i(0)                ... i(N)
///
/// N requests
///
#[tracing::instrument]
pub fn extract_segments(start: i32, stop: i32) -> eyre::Result<Vec<i32>> {
    trace!("enter");

    let beg_hour = start - (start % ONE_HOUR);
    let end_hour = stop - (stop % ONE_HOUR);

    let mut v = vec![];
    let mut i = beg_hour;
    while i <= end_hour {
        v.push(i);
        i += ONE_HOUR;
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(3600,  3650, &[0])]
    #[case(3600,  7200, &[0, 1])]
    #[case(3610,  7200, &[0, 1])]
    fn test_extract_segment(#[case] fr: i32, #[case] to: i32, #[case] res: &[i32]) {
        assert_eq!(res, extract_segments(fr, to)?);
    }
}
