//! This example demonstrates multiple methods to calculate the distance between
//! two geographical points specified by their latitude and longitude. It
//! includes implementations using the Haversine formula, the Spherical Law
//! of Cosines, and multiple distance calculations provided by the `geo` crate.
//!  
//! Additionally, it shows how to interact with a ClickHouse database to use
//! its `geoDistance` function for distance computation. The example uses
//! command-line arguments to input the coordinates and outputs the results of all methods.
//!
use clap::Parser;
use geo::point;
use geo::prelude::*;
use klickhouse::{ClientOptions, QueryBuilder, RawRow};

/// Earth radius in meters
const R: f64 = 6_371_088.0;

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(short = 'O', long, default_value = "false")]
    pub offline: bool,
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
    /// Calculates the great-circle distance between two geographical points
    /// using the Haversine formula.
    ///
    /// # Parameters
    ///
    /// * `self` - The first geographical point, represented as `Point`.
    /// * `other` - The second geographical point, represented as `Point`.
    ///
    /// # Returns
    ///
    /// * A `f64` value representing the distance in meters.
    ///
    /// # Formula
    ///
    /// The Haversine formula is used to calculate the great-circle distance
    /// between two points on a sphere:
    ///
    /// * d_lat = (lat2 - lat1).to_radians()
    /// * d_lon = (lon2 - lon1).to_radians()
    /// * a = sin²(d_lat / 2) + cos(lat1) * cos(lat2) * sin²(d_lon / 2)
    /// * c = 2 * atan2(√a, √(1-a))
    /// * distance = Earth's radius * c
    ///
    /// This method assumes that the Earth is a perfect sphere.
    ///
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

    /// Calculates the great-circle distance between two geographical points
    /// using the Spherical Law of Cosines.
    ///
    /// # Parameters
    ///
    /// * `self` - The first geographical point, represented as `Point`.
    /// * `other` - The second geographical point, represented as `Point`.
    ///
    /// # Returns
    ///
    /// * A `f64` value representing the distance in meters.
    ///
    /// # Formula
    ///
    /// The Spherical Law of Cosines is used to calculate the great-circle
    /// distance between two points on a sphere:
    ///
    /// * a = sin(lat1) * sin(lat2) + cos(lat1) * cos(lat2) * cos(d_lon)
    /// * angle = acos(a)
    /// * distance = Earth's radius * angle
    ///
    fn spherical_law_of_cosines_distance(&self, other: &Point) -> f64 {
        //let d_lat = other.latitude.to_radians() - self.latitude.to_radians();
        let d_lon = other.longitude.to_radians() - self.longitude.to_radians();

        let a = (self.latitude.to_radians()).sin() * (other.latitude.to_radians()).sin()
            + (self.latitude.to_radians()).cos()
                * (other.latitude.to_radians()).cos()
                * d_lon.cos();

        let c = a.acos();

        R * c
    }
}

/// Computes the geodesic distance between two geographical points
/// using ClickHouse's `geoDistance` function.
///
/// This function connects to a ClickHouse database specified by
/// environment variables (`KLICKHOUSE_URL`, `CLICKHOUSE_DB`,
/// `CLICKHOUSE_USER`, and `CLICKHOUSE_PASSWD`) and executes a query
/// to calculate the distance using ClickHouse's optimized geographical
/// distance calculation feature.
///
/// # Parameters
///
/// * `point1` - The first geographical point, represented as a `Point`.
/// * `point2` - The second geographical point, represented as a `Point`.
///
/// # Returns
///
/// A `Result` containing:
/// * `Ok(f64)` - The computed distance in meters if the query completes successfully.
/// * `Err(eyre::Error)` - If an error occurs during connection, query execution, or data extraction.
///
/// # Environment Variables
///
/// To use this function, ensure the following environment variables are set:
/// * `KLICKHOUSE_URL`: The URL of the ClickHouse database.
/// * `CLICKHOUSE_DB`: The name of the database to use for the query.
/// * `CLICKHOUSE_USER`: The username for database authentication.
/// * `CLICKHOUSE_PASSWD`: The password for database authentication.
///
/// # Example
///
/// ```rust
/// use eyre::Result;
/// let point1 = Point {
///     latitude: 48.8566,  // Latitude of Paris
///     longitude: 2.3522,  // Longitude of Paris
/// };
/// let point2 = Point {
///     latitude: 51.5074,  // Latitude of London
///     longitude: -0.1278, // Longitude of London
/// };
/// let distance = ch_distance(point1, point2).await?;
/// println!("Distance (ClickHouse): {:.2} meters", distance);
/// ```
///
/// # Errors
///
/// This function returns an error if:
/// * The required environment variables are not set.
/// * There is an issue connecting to the ClickHouse database.
/// * The query fails or returns invalid data.
///
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
            ..Default::default()
        },
    )
    .await?;

    let q = QueryBuilder::new("SELECT geoDistance($1,$2,$3,$4) AS dist")
        .arg(point1.longitude)
        .arg(point1.latitude)
        .arg(point2.longitude)
        .arg(point2.latitude);
    dbg!(&q.clone().finalize());
    let mut val = client.query_one::<RawRow>(q).await?;
    let val: f64 = val.get(0);
    dbg!(&val);
    Ok(val.into())
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
    let geo_g = Geodesic.distance(p1, p2);
    let geo_h = Haversine.distance(p1, p2);
    let geo_vin = p1.vincenty_distance(&p2)?;

    let ch_dist = if !opts.offline {
        ch_distance(point1, point2).await?
    } else {
        0.0_f64
    };

    println!("Distance between\n  {:?}\nand\n  {:?}", p1, p2);
    println!(
        "Distances:\n\
        {:.2} m haversines\n\
        {:.2} m (sin/cos)\n\
        {:.2} m geo::geodesic\n\
        {:.2} m geo::haversines\n\
        {:.2} m geo::vincenty\n\
        {:.2} m clickhouse\n",
        d_h, d_s, geo_g, geo_h, geo_vin, ch_dist
    );
    Ok(())
}
