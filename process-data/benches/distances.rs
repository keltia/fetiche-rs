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
            black_box(p1.geodesic_distance(&p2));
        })
    });
}

fn geo_haversines(c: &mut Criterion) {
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
    let rt = tokio::runtime::Handle::current();
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

    c.bench_function("klickhouse", |b| {
        b.to_async(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap(),
        )
            .iter(|| async {
                let _ = ch_calc_distance(client.clone(), &point1, &point2).await;
            });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = self_haversines, self_cosinuses, geo_geodesic, geo_haversines, geo_vincenty, ch_geodistance
}

criterion_main!(benches);
