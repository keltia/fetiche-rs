//! This benchmark file measures the performance of various distance calculation methods
//! between two geographical points represented by latitude and longitude.
//!
//! # Benchmarking Functions
//!
//! This file uses the `Criterion` crate for benchmarking, providing both synchronous and 
//! asynchronous execution contexts where required.
//!
//! ## Self-Implemented Distance Methods
//!
//! 1. **self::haversines**
//!    - Benchmarks the `haversine_distance` function implemented on the `Pt` struct.
//!    - The haversine formula calculates the great-circle distance between two points 
//!      on a sphere using their longitudes and latitudes.
//!
//! 2. **self::sincosines**
//!    - Benchmarks the `spherical_law_of_cosines_distance` function on the `Pt` struct.
//!    - This method uses the spherical law of cosines for computing the distance between two 
//!      geographic points.
//!
//! ## Geo Crate Methods
//!
//! The `geo` crate provides several methods for geodesic calculations:
//!
//! 3. **geo::geodesic**
//!    - Benchmarks the `Geodesic::distance` method from the `geo` crate, which computes distances 
//!      using geodesic principles.
//!
//! 4. **geo::haversines**
//!    - Benchmarks the `Haversine::distance` method from the `geo` crate, using the haversine formula.
//!
//! 5. **geo::vincenty**
//!    - Benchmarks the `vincenty_distance` method from the `geo` crate to compute distances based on 
//!      the Vincenty inverse formula for ellipsoids.
//!
//! ## Klickhouse Database Method
//!
//! 6. **klickhouse**
//!    - Benchmarks the `geoDistance` function provided by the `klickhouse` library, which performs 
//!      distance calculations directly on the ClickHouse database server via SQL queries.
//!    - This benchmark uses asynchronous execution and connects to ClickHouse using environment variables
//!      to retrieve the necessary credentials (KLICKHOUSE_URL, CLICKHOUSE_DB, CLICKHOUSE_USER, and CLICKHOUSE_PASSWD).
//!
//! # Usage
//!
//! To run the benchmarks, use the following command:
//! ```sh
//! cargo bench
//! ```
//!
//! Ensure that required environment variables are set for the ClickHouse benchmarks, and the `klickhouse` 
//! server is accessible.
//!
//! # Structs
//!
//! - `Pt`: A helper struct holding latitude and longitude coordinates. It provides methods for computing 
//! distances using both the haversine formula (`haversine_distance`) and the spherical law of cosines 
//! (`spherical_law_of_cosines_distance`).
//!
//! # Setup
//!
//! The `setup` function generates two `Pt` instances representing two geographical points near Paris, France.
//!
//! # External Dependencies
//!
//! - `Criterion`: For benchmarking frameworks.
//! - `geo`: For geospatial calculations.
//! - `klickhouse`: For interfacing with ClickHouse databases to benchmark SQL-based distance computation.
//! - `tokio`: For managing asynchronous operations.
//! - `std::env`: For environment variable retrieval in the `klickhouse` benchmark.
//!
use criterion::async_executor::FuturesExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use geo::point;
use geo::prelude::*;
use klickhouse::{Client, ClientOptions, QueryBuilder, RawRow};

struct Pt {
    latitude: f64,
    longitude: f64,
}

impl Pt {
    const R: f64 = 6_371_088.0; // Earth radius in meters

    fn haversine_distance(&self, other: &Pt) -> f64 {
        let d_lat = (other.latitude - self.latitude).to_radians();
        let d_lon = (other.longitude - self.longitude).to_radians();

        let a = (d_lat / 2.0).sin() * (d_lat / 2.0).sin()
            + self.latitude.to_radians().cos()
            * other.latitude.to_radians().cos()
            * (d_lon / 2.0).sin()
            * (d_lon / 2.0).sin();

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        Pt::R * c
    }

    fn spherical_law_of_cosines_distance(&self, other: &Pt) -> f64 {
        let d_lat = other.latitude.to_radians() - self.latitude.to_radians();
        let d_lon = other.longitude.to_radians() - self.longitude.to_radians();

        let a = (self.latitude.to_radians()).sin() * (other.latitude.to_radians()).sin()
            + (self.latitude.to_radians()).cos()
            * (other.latitude.to_radians()).cos()
            * d_lon.cos();

        let c = a.acos();

        Pt::R * c
    }
}

fn setup() -> (Pt, Pt) {
    let point1 = Pt {
        latitude: 48.573174,
        longitude: 2.319671,
    };
    let point2 = Pt {
        latitude: 48.566757,
        longitude: 2.303015,
    };
    (point1, point2)
}

fn self_haversines(c: &mut Criterion) {
    let (point1, point2) = setup();

    c.bench_function("self::haversines", move |b| {
        b.iter(|| {
            black_box(point1.haversine_distance(&point2));
        })
    });
}

fn self_cosinuses(c: &mut Criterion) {
    let (point1, point2) = setup();

    c.bench_function("self::sincosines", |b| {
        b.iter(|| {
            point1.spherical_law_of_cosines_distance(black_box(&point2));
        })
    });
}

fn geo_geodesic(c: &mut Criterion) {
    let (point1, point2) = setup();

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);

    c.bench_function("geo::geodesic", |b| {
        b.iter(|| {
            black_box(Geodesic::distance(p1, p2));
        })
    });
}

fn geo_haversines(c: &mut Criterion) {
    let (point1, point2) = setup();

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);

    c.bench_function("geo::haversines", |b| {
        b.iter(|| {
            black_box(Haversine::distance(p1, p2));
        })
    });
}

fn geo_vincenty(c: &mut Criterion) {
    let (point1, point2) = setup();

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);

    c.bench_function("geo::vincenty", |b| {
        b.iter(|| {
            black_box(p1.vincenty_distance(&p2).unwrap());
        })
    });
}

async fn ch_calc_distance(client: Client, point1: &Pt, point2: &Pt) -> f64 {
    let q = QueryBuilder::new("SELECT geoDistance($1,$2,$3,$4) AS dist")
        .arg(point1.longitude)
        .arg(point1.latitude)
        .arg(point2.longitude)
        .arg(point2.latitude);

    let mut res = client.query_one::<RawRow>(q).await.unwrap();
    let val: f64 = res.get("dist");
    val
}

fn ch_geodistance(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { inner_ch_geodistance(c).await });
}

async fn inner_ch_geodistance(c: &mut Criterion) {
    let (point1, point2) = setup();

    let url = std::env::var("KLICKHOUSE_URL").unwrap();
    let db = std::env::var("CLICKHOUSE_DB").unwrap();
    let user = std::env::var("CLICKHOUSE_USER").unwrap();
    let pwd = std::env::var("CLICKHOUSE_PASSWD").unwrap();

    let client = Client::connect(
        url,
        ClientOptions {
            username: user,
            password: pwd,
            default_database: db,
            ..Default::default()
        },
    )
        .await
        .unwrap();

    c.bench_function("klickhouse", move |b| {
        b.to_async(FuturesExecutor).iter(|| async {
            black_box(ch_calc_distance(client.clone(), &point1, &point2).await);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = self_haversines, self_cosinuses, geo_geodesic, geo_haversines, geo_vincenty, ch_geodistance
}

criterion_main!(benches);
