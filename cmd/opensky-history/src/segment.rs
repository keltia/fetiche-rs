use tracing::trace;

const ONE_HOUR: i64 = 3_600;

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
pub fn extract_segments(start: i64, stop: i64) -> eyre::Result<Vec<i64>> {
    trace!("enter");

    let beg_hour = start.div_euclid(ONE_HOUR) * ONE_HOUR;
    let end_hour = stop.div_euclid(ONE_HOUR) * ONE_HOUR;

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
    #[case(3600, 3650, & [3600])]
    #[case(3600, 7200, & [3600, 7200])]
    #[case(3610, 7200, & [3600, 7200])]
    #[case(3610, 7230, & [3600, 7200])]
    #[case(3610, 11000, & [3600, 7200, 10800])]
    #[case(2_147_486_400, 2_147_490_000, &[2_147_486_400, 2_147_490_000])]
    fn test_extract_segment(#[case] fr: i64, #[case] to: i64, #[case] res: &[i64]) {
        assert_eq!(res, extract_segments(fr, to).unwrap());
    }
}
