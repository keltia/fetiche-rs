use csv::ReaderBuilder;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Point {
    longitude: f64,
    latitude: f64,
    altitude: u32,
}


fn main() -> eyre::Result<()> {
    let fname = "points.csv";
    let mut csv = ReaderBuilder::new().has_headers(true).from_path(fname)?;
    let rows = csv.deserialize().map(|rec| {
        let rec: Point = rec.unwrap();
        (rec.longitude, rec.latitude, rec.altitude as f64)
    }).collect::<Vec<_>>();
    dbg!(&rows);

    let

        Ok(())
}
