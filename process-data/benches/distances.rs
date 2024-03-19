use criterion::{black_box, criterion_group, criterion_main, Criterion};
use duckdb::{Connection, params};
use geo::point;
use geo::prelude::*;

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
    eprintln!("haversines");
    let (point1, point2) = setup();

    c.bench_function("self::haversines", move |b| {
        b.iter(|| {
            black_box(point1.haversine_distance(&point2));
        })
    });
}

fn self_cosinuses(c: &mut Criterion) {
    eprintln!("cosinuses");
    let (point1, point2) = setup();

    c.bench_function("self::sincosines", |b| {
        b.iter(|| {
            point1.spherical_law_of_cosines_distance(black_box(&point2));
        })
    });
}

fn geo_geodesic(c: &mut Criterion) {
    eprintln!("geodesic");
    let (point1, point2) = setup();

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);

    c.bench_function("geo::geodesic", |b| {
        b.iter(|| {
            black_box(p1.geodesic_distance(&p2));
        })
    });
}

fn geo_haversines(c: &mut Criterion) {
    eprintln!("haversines");
    let (point1, point2) = setup();

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);

    c.bench_function("geo::haversines", |b| {
        b.iter(|| {
            black_box(p1.haversine_distance(&p2));
        })
    });
}

fn geo_vincenty(c: &mut Criterion) {
    eprintln!("vincenty");
    let (point1, point2) = setup();

    let p1 = point!(x: point1.longitude, y: point1.latitude);
    let p2 = point!(x: point2.longitude, y: point2.latitude);

    c.bench_function("geo::vincenty", |b| {
        b.iter(|| {
            black_box(p1.vincenty_distance(&p2).unwrap());
        })
    });
}

fn duckdb_calc(dbh: &Connection, p1: &Pt, p2: &Pt) {
    dbh.execute("SELECT ST_Distance_Spheroid(ST_Point(?, ?), ST_Point(?, ?))",
                params![p1.latitude, p1.longitude, p2.latitude, p2.longitude]).unwrap();
}

fn duckdb_spheroid(c: &mut Criterion) {
    eprint!("duckdb");
    let (point1, point2) = setup();

    let dbh = duckdb::Connection::open_in_memory().unwrap();
    dbh.execute("LOAD spatial", []).unwrap();

    c.bench_function("duckdb_spheroid", |b| {
        b.iter(|| {
            black_box(duckdb_calc(&dbh, &point1, &point2))
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = self_haversines, self_cosinuses, geo_geodesic, geo_haversines, geo_vincenty, duckdb_spheroid
}

criterion_main!(benches);
