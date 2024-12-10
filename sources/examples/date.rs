use eyre::Result;
use polars::datatypes::Int64Chunked;
use polars::prelude::{Column, CsvParseOptions, CsvReadOptions, IntoColumn, SerReader};
use std::io::Cursor;

fn main() -> Result<()> {
    let data = r##"journey,ident,model,source,location,timestamp,latitude,longitude,altitude,elevation,gps,rssi,home_lat,home_lon,home_height,speed,heading,station_name,station_latitude,station_longitude
72709,F6Z9C242V003PQBK,"DJI Mini4 Pro",as,2527943,"2024-12-09 11:04:59",34.710918,32.571717,108,92,,,34.711101,32.571637,16,0,338,0QRDKC2R03J32P,34.718506,32.475510
72706,L2T0023RB7,"Mini 2 SE",as,2527854,"2024-12-09 06:11:48",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
72706,L2T0023RB7,"Mini 2 SE",as,2527855,"2024-12-09 06:11:58",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
72706,L2T0023RB7,"Mini 2 SE",as,2527856,"2024-12-09 06:11:59",48.156054,16.350434,312,201,,,48.155487,16.350984,113,0,343,0QRDKC2R038370,48.104234,16.589570
"##;

    // We need to fix the timestamp field.
    //
    let cur = Cursor::new(&data);
    let opts = CsvParseOptions::default().with_try_parse_dates(false);
    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .with_parse_options(opts)
        .into_reader_with_file_handle(cur)
        .finish()?;

    let r = df.apply("timestamp", into_timestamp)?;
    dbg!(r.select_columns(["timestamp"]));

    Ok(())
}

fn into_timestamp(col: &Column) -> Column {
    col.str()
        .unwrap()
        .into_iter()
        .map(|d: Option<&str>| d.map(|d: &str| dateparser::parse(d).unwrap().timestamp()))
        .collect::<Int64Chunked>()
        .into_column()
}
