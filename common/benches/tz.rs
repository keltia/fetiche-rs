//! Benchmark for timezone lookup strategies:
//! - Static `std::sync::LazyLock` shared spatial index (current implementation)
//! - Per-lookup `tzf_rs::DefaultFinder::new()` (un-cached index construction)
//! - Memoized lookup using `cached` (quantized cache hits vs cache misses)
//! - Multi-location spatial queries across major global cities

use std::hint::black_box;
use std::sync::LazyLock;

use cached::proc_macro::cached;
use divan::Bencher;
use fetiche_common::find_tz;
use tzf_rs::DefaultFinder;

fn main() {
    divan::main();
}

static BENCH_FINDER: LazyLock<DefaultFinder> = LazyLock::new(DefaultFinder::new);

const PARIS: (f64, f64) = (48.8566, 2.3522);
const NEW_YORK: (f64, f64) = (40.7128, -74.0060);
const TOKYO: (f64, f64) = (35.6762, 139.6503);
const SYDNEY: (f64, f64) = (-33.8688, 151.2093);
const LONDON: (f64, f64) = (51.5074, -0.1278);

const CITIES: [(f64, f64); 5] = [PARIS, NEW_YORK, TOKYO, SYDNEY, LONDON];

/// Benchmark full `find_tz` using static `LazyLock` (index + timezone offset).
#[divan::bench]
fn lazylock_find_tz(bencher: Bencher) {
    let (lat, lon) = PARIS;
    bencher.bench_local(move || {
        black_box(find_tz(black_box(lat), black_box(lon)).unwrap())
    });
}

/// Benchmark raw spatial polygon query using static `LazyLock` (no Jiff offset).
#[divan::bench]
fn lazylock_raw_polygon(bencher: Bencher) {
    let (lat, lon) = PARIS;
    bencher.bench_local(move || {
        let name = BENCH_FINDER.get_tz_name(black_box(lon), black_box(lat));
        black_box(name)
    });
}

/// Benchmark recreating `DefaultFinder::new()` on every call (prior un-cached behavior).
#[divan::bench]
fn recreate_default_finder(bencher: Bencher) {
    let (lat, lon) = PARIS;
    bencher.bench_local(move || {
        let finder = DefaultFinder::new();
        let name = finder.get_tz_name(black_box(lon), black_box(lat)).to_string();
        black_box(name)
    });
}

/// Memoized timezone lookup using `cached` with 1000-entry LRU cache.
/// Coordinates are quantized to 4 decimal places (approx. 11 meters precision).
#[cached(max_size = 1000)]
fn cached_tz_lookup(lat_q: i64, lon_q: i64) -> String {
    let lat = lat_q as f64 / 10000.0;
    let lon = lon_q as f64 / 10000.0;
    BENCH_FINDER.get_tz_name(lon, lat).to_string()
}

/// Benchmark memoized lookup on repeat hits (same coordinate).
#[divan::bench]
fn cached_hit(bencher: Bencher) {
    let lat_q = (PARIS.0 * 10000.0).round() as i64;
    let lon_q = (PARIS.1 * 10000.0).round() as i64;
    // Prime the cache
    let _ = cached_tz_lookup(lat_q, lon_q);

    bencher.bench_local(move || {
        black_box(cached_tz_lookup(black_box(lat_q), black_box(lon_q)))
    });
}

/// Benchmark memoized lookup on misses (simulating streaming unique telemetry coordinates).
#[divan::bench]
fn cached_miss_stream(bencher: Bencher) {
    let mut counter = 0i64;
    bencher.bench_local(move || {
        counter = counter.wrapping_add(1);
        // Varying latitude slightly to force cache misses
        let lat_q = 480000 + (counter % 50000);
        let lon_q = 20000 + (counter % 50000);
        black_box(cached_tz_lookup(black_box(lat_q), black_box(lon_q)))
    });
}

/// Benchmark querying multiple global cities sequentially using `LazyLock`.
#[divan::bench]
fn lazylock_multi_cities(bencher: Bencher) {
    bencher.bench_local(move || {
        for &(lat, lon) in &CITIES {
            let res = find_tz(black_box(lat), black_box(lon)).unwrap();
            black_box(res);
        }
    });
}
