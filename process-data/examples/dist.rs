use clap::Parser;
use duckdb::params;
use geo::point;
use geo::prelude::*;
use clickhouse_rs::{Pool, types::Block};
use parquet2::FallibleStreamingIterator;

/// Earth radius in meters
const R: f64 = 6_371_088.0;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(short = 'p', long)]
    pub password: Option<String>,
    pub lat1: f64,
    pub lon1: f64,
    pub lat2: f64,
    pub lon2: f64,
}

#[derive(Debug)]
struct Point {
    latitude: f64,
    longitude: f64,
}

impl Point {
    fn haversine_distance(&self, other: &Point) -> f64 {
        let d_lat = (other.latitude - self.latitude).to_radians();
        let d_lon = (other.longitude - self.longitude).to_radians();

        let a = (d_lat / 2.0).sin() * (d_lat / 2.0).sin()
            + self.latitude.to_radians().cos()
            * other.latitude.to_radians().cos()
            * (d_lon / 2.0).sin()
            * (d_lon / 2.0).sin();

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        R * c
    }

    fn spherical_law_of_cosines_distance(&self, other: &Point) -> f64 {
        let d_lat = other.latitude.to_radians() - self.latitude.to_radians();
        let d_lon = other.longitude.to_radians() - self.longitude.to_radians();

        let a = (self.latitude.to_radians()).sin() * (other.latitude.to_radians()).sin()
            + (self.latitude.to_radians()).cos()
            * (other.latitude.to_radians()).cos()
            * d_lon.cos();

        let c = a.acos();

        R * c
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let opts: Opts = Opts::parse();

    let point1 = Point {
        latitude: opts.lat1,
        longitude: opts.lon1,
    };
    let point2 = Point {
        latitude: opts.lat2,
        longitude: opts.lon2,
    };

    let d_h = point1.haversine_distance(&point2);
    let d_s = point1.spherical_law_of_cosines_distance(&point2);

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);
    let geo_g = p1.geodesic_distance(&p2);
    let geo_h = p1.haversine_distance(&p2);
    let geo_vin = p1.vincenty_distance(&p2)?;

    let dbh = duckdb::Connection::open_in_memory()?;
    dbh.execute("LOAD spatial", [])?;

    let distduck: f64 = dbh.query_row("SELECT ST_Distance_Spheroid(ST_Point(?, ?), ST_Point(?, ?))",
                                      params![point1.latitude, point1.longitude, point2.latitude, point2.longitude], |row| {
            Ok(row.get_unwrap(0))
        },
    )?;

    let url = format!("tcp://default:{}@100.92.250.113:9000/default?compression=lz4&readonly=1", opts.password.unwrap());
    let pool = Pool::new(url);
    let query = format!("SELECT geoDistance({},{},{},{}) AS dist", point1.longitude, point1.latitude, point2.longitude, point2.latitude);

    let r = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    let done = pool
        .get_handle()
        .and_then(move |c| c.query(query).fetch_all())
        .and_then(move |(_, block)| {
            let dist = block.rows().into_iter().for_each(|r| {
                let dist: f64 = r.get("dist")?;
                eprintln!("dist = {}", dist);
                Ok(dist)
            }
            Ok(dist)
        })
        .map_err(|err| eprintln!("DB error: {}", err))?;


    println!("Distance between\n  {:?}\nand\n  {:?}", p1, p2);
    println!(
        "Distances:\n\
        {:.2} m haversines\n\
        {:.2} m (sin/cos)\n\
        {:.2} m geo::geodesic\n\
        {:.2} m geo::haversines\n\
        {:.2} m geo::vincenty\n\
        {:.2} m duckdb:speh\n",
        d_h, d_s, geo_g, geo_h, geo_vin, distduck
    );
    Ok(())
}
