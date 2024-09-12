use clap::Parser;
use geo::point;
use geo::prelude::*;
use klickhouse::{ClientOptions, QueryBuilder, Row};

#[cfg(feature = "duckdb")]
use duckdb::params;

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

#[derive(Copy, Clone, Debug)]
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

async fn ch_distance(point1: Point, point2: Point) -> eyre::Result<f64> {
    let url = std::env::var("KLICKHOUSE_URL")?;
    let db = std::env::var("CLICKHOUSE_DB")?;
    let user = std::env::var("CLICKHOUSE_USER")?;
    let pwd = std::env::var("CLICKHOUSE_PASSWD")?;

    let client = klickhouse::Client::connect(
        url,
        ClientOptions {
            username: user,
            password: pwd,
            default_database: db,
        },
    )
        .await?;

    #[derive(Debug, Row)]
    struct Ans {
        dist: f64,
    }
    let q = QueryBuilder::new("SELECT geoDistance($1,$2,$3,$4) AS dist")
        .arg(point1.longitude).arg(point1.latitude).arg(point2.longitude).arg(point2.latitude);
    dbg!(q.clone().finalize()?);
    let val = client.query_one::<Ans>(q).await?;
    dbg!(&val);
    Ok(val.dist.into())
}

#[cfg(feature = "duckdb")]
async fn dd_distance(point1: Point, point2: Point) -> eyre::Result<f64> {
    let dbh = duckdb::Connection::open_in_memory()?;
    dbh.execute("LOAD spatial", [])?;

    let dist_duck: f64 = dbh.query_row("SELECT ST_Distance_Spheroid(ST_Point(?, ?), ST_Point(?, ?))",
                                       params![point1.latitude, point1.longitude, point2.latitude, point2.longitude], |row| {
            Ok(row.get_unwrap(0))
        },
    )?;
    Ok(dist_duck)
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

    let dist_duck: f64 = 0.;
    #[cfg(feature = "duckdb")]
    let dist_duck = dd_distance(point1, point2).await?;
    let ch_dist = ch_distance(point1, point2).await?;

    println!("Distance between\n  {:?}\nand\n  {:?}", p1, p2);
    println!(
        "Distances:\n\
        {:.2} m haversines\n\
        {:.2} m (sin/cos)\n\
        {:.2} m geo::geodesic\n\
        {:.2} m geo::haversines\n\
        {:.2} m geo::vincenty\n\
        {:.2} m duckdb:speh\n\
        {:.2} m clickhouse\n",
        d_h, d_s, geo_g, geo_h, geo_vin, dist_duck, ch_dist
    );
    Ok(())
}
