use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polars::io::SerReader;
use polars::prelude::{JsonFormat, JsonReader};
use std::io::{BufRead, Cursor};

use fetiche_formats::avionix::CubeData;

const DATA: &str = r##"{"uti":1732541258,"dat":"2024-11-25 13:27:38.190766816","tim":"13:27:38.190766816","hex":"44cc43","fli":"BEL7LX","lat":49.25520040221133,"lon":2.238608912417763,"gda":"A","src":"A","alt":32225,"altg":32375,"hgt":150,"spd":562,"cat":"A3","squ":"1000","vrt":-448,"trk":31.767733,"mop":2,"lla":1,"tru":2031,"dbm":-68}
{"uti":1732541270,"dat":"2024-11-25 13:27:40.190766816","tim":"13:27:38.190766816","hex":"44cc44","fli":"BEL7LX","lat":49.25520040221133,"lon":2.238608912417763,"gda":"A","src":"A","alt":32225,"altg":32375,"hgt":150,"spd":562,"cat":"A3","squ":"1000","vrt":-448,"trk":31.767733,"mop":2,"lla":1,"tru":2031,"dbm":-68}
{"uti":1732541273,"dat":"2024-11-25 13:27:40.190766816","tim":"13:27:38.190766819","hex":"43cc44","fli":"BEL7LX","lat":49.25520040221133,"lon":2.238608912417763,"gda":"A","src":"A","alt":32225,"altg":32375,"hgt":150,"spd":562,"cat":"A3","squ":"1000","vrt":-448,"trk":31.767733,"mop":2,"lla":1,"tru":2031,"dbm":-68}
{"uti":1732541284,"dat":"2024-11-25 13:27:40.190766816","tim":"13:27:38.190766822","hex":"44cd44","fli":"BE8ALX","lat":49.25520040221133,"lon":2.238608912417763,"gda":"A","src":"A","alt":32225,"altg":32375,"hgt":150,"spd":562,"cat":"A3","squ":"1000","vrt":-448,"trk":31.767733,"mop":2,"lla":1,"tru":2031,"dbm":-68}
{"uti":1732541290,"dat":"2024-11-25 13:27:40.190766816","tim":"13:27:38.190766830","hex":"44cc44","fli":"BEL7LX","lat":49.25520040221133,"lon":2.238608912417763,"gda":"A","src":"A","alt":32225,"altg":32375,"hgt":150,"spd":562,"cat":"A3","squ":"1000","vrt":-448,"trk":31.767733,"mop":2,"lla":1,"tru":2031,"dbm":-68}
"##;

fn read_polars_raw(c: &mut Criterion) {
    c.bench_function("read_polars_raw", |b| {
        b.iter(|| {
            let cur = Cursor::new(DATA);
            let _df = black_box(
                JsonReader::new(cur)
                    .with_json_format(JsonFormat::JsonLines)
                    .finish()
                    .unwrap(),
            );
        })
    });
}

fn read_serde_json(c: &mut Criterion) {
    c.bench_function("read_serde_json", |b| {
        b.iter(|| {
            let _a: Vec<CubeData> = black_box(
                DATA.lines()
                    .map(|r| {
                        let r: CubeData = serde_json::from_str(r).unwrap();
                        r
                    })
                    .collect(),
            );
        })
    });
}

fn read_serde_json_cursor(c: &mut Criterion) {
    c.bench_function("serde_json_cursor", |b| {
        b.iter(|| {
            let cur = Cursor::new(DATA);
            let _data: Vec<CubeData> = black_box(
                cur.lines()
                    .map(|r| {
                        let r: CubeData = serde_json::from_str(r.unwrap().as_str()).unwrap();
                        r
                    })
                    .collect(),
            );
        })
    });
}

criterion_group!(
    benches,
    read_polars_raw,
    read_serde_json,
    read_serde_json_cursor
);

criterion_main!(benches);
