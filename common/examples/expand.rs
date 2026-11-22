use jiff::ToSpan;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;

pub fn expand_interval_civil(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
    let mut d = begin;
    let mut intv = vec![];

    let day = 1.days();
    while d < end {
        intv.push(d);
        d = d.checked_add(day).expect("overflow");
    }
    Ok(intv)
}

pub fn expand_interval_jiff_series(begin: DateTime, end: DateTime) -> eyre::Result<Vec<DateTime>> {
    let intv = begin
        .series(1.days())
        .take_while(|&ts| ts < end)
        .collect::<Vec<_>>();
    Ok(intv)
}

fn main() -> eyre::Result<()> {
    let begin = "2024-01-01".parse()?;
    let end = "2025-01-01".parse()?;

    let r1 = expand_interval_civil(begin, end)?;
    eprintln!("vec(jiff) = {}", r1.len());

    let r3 = expand_interval_jiff_series(begin, end)?;
    eprintln!("vec(series) = {}", r3.len());

    let tz = TimeZone::UTC;
    let r1 = r1
        .into_iter()
        .map(|x| x.to_zoned_datetime(tz.clone()).unwrap().to_string())
        .collect::<Vec<_>>();
    let r3 = r3
        .into_iter()
        .map(|x| x.to_zoned_datetime(tz.clone()).unwrap().to_string())
        .collect::<Vec<_>>();

    if r1 != r3 {
        eprintln!("r1 != r3");
    }

    assert_eq!(r1, r3);
    Ok(())
}
