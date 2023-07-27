// start = NNNNNN
// stop = MMMMMM
//
// i(0) => beg_hour = NNNNNN
// i(N) => end_hour = MMMMMM - (MMMMMM mod 3600)
//
// N =  (MMMMMM - NNNNNN) / 3600
//
// thus
//
// [beg_hour <= start] ... [end_hour <= stop]
// i(0)                ... i(N)
//
// N requests
//

use anyhow::Result;
use inline_python::{python, Context};

pub fn extract_segments(start: i32, stop: i32) -> Result<Vec<i32>> {
    let beg_hour = start - (start % 3600);
    let end_hour = stop - (stop % 3600);

    let mut v = vec![];
    let mut i = beg_hour;
    while i <= end_hour {
        v.push(i);
        i += 3600;
    }
    Ok(v)
}

fn main() -> Result<()> {
    // 20230621
    let start = 1687354159_i32;
    // 20230627
    let end = 1687872573_i32;

    let ctx = Context::new();

    let v = extract_segments(start, end)?;
    println!("{} segments", v.len());
    println!("{:?}", v);

    let res = python! {print "hello"};

    Ok(())
}
