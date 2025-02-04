//! # Example: Distances in 3D Space
//!
//! This example demonstrates different methods to calculate distances
//! between points in 3D space, using both simple trigonometry and geodesic calculations.
//!
//! ## Overview
//! The file defines the `Point` structure for representing 3D coordinates,
//! and implements distance calculations leveraging the `geo` and `rust_3d` crates.
//!
//! It includes two modules:
//!
//! - `roberto`: Contains basic trigonometric and geographic distance calculations.
//! - `gravis`: Provides a more thorough approach to transforming coordinates 
//!   using geocentric latitude and Earth radius calculations, with enhanced precision.
//!
//! ## Coordinate Conventions
//! - Geographic coordinates are represented as latitude (`lat`), longitude (`lon`),
//!   and altitude (`alt`).
//! - Points may also be converted to 3D Cartesian coordinates for specific computations.
//!
//! ## Usage
//! - To understand the differences between various distance calculation methods, 
//!   compare values printed using `roberto::distances` and `gravis::distances`.
//! - Change input points for testing various scenarios.
//!

// Point3D is ( x = lon, y = lat, z = alt )
// Point is ( lat, lon alt )
// point! is ( x = lon, y = lat )

#[derive(Copy, Clone, Debug)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
}

impl Point {
    pub fn new(lat: f64, lon: f64, alt: f64) -> Self {
        Self { lat, lon, alt }
    }
}

mod roberto {
    use geo::{point, Distance, Geodesic, Haversine};
    use rust_3d::{dist_2d, dist_3d, IsTransFormableTo2D, Point2D, Point3D};

    use crate::Point;

    /// 1 deg = 59.9952 nm or 111.1111 km
    const R: f64 = 111_111.11;

    /// Basic trig, no geo stuff
    ///
    pub fn trig_distances(a: &Point3D, b: &Point3D) -> (f64, f64) {
        let a2: Point2D = a.transform_to_2d();
        let b2: Point2D = b.transform_to_2d();

        let d2 = dist_2d(&a2, &b2);
        let d3 = dist_3d(a, b);
        (d2, d3)
    }

    pub fn basic() -> eyre::Result<()> {
        let a3 = Point3D::new(0., 0., 0.);
        let b3 = Point3D::new(1., 1., 1.);
        let c3 = Point3D::new(1., 1., -1.);

        let (ab2, ab3) = trig_distances(&a3, &b3);
        let (ac2, ac3) = trig_distances(&a3, &c3);

        println!("=====");
        println!("Basic 2D/3D:");
        println!("  Dist(A,B) = 2D=({:.2}) or 3D=({:.2})", ab2, ab3);
        println!("  Dist(A,C) = 2D=({:.2}) or 3D=({:.2})", ac2, ac3);

        Ok(())
    }

    pub fn distances(drone: Point, home: Point) -> eyre::Result<()> {
        println!("===== roberto =====");

        // Real lat/lon
        //
        println!("  A={:?}", drone);
        println!("  B={:?}", home);

        println!("-----");

        // Basic trig.
        //
        let d2calc = (drone.lon - home.lon).powi(2) + (drone.lat - home.lat).powi(2);
        let dcalc = d2calc.sqrt() * R;

        // Geo stuff, distances are in meters
        //
        let drone2 = point!(x: drone.lon, y: drone.lat);
        let home2 = point!(x: home.lon, y: home.lat);
        let dcalc2g = Geodesic.distance(drone2, home2);
        let dcalc2h = Haversine.distance(drone2, home2);
        println!(
            "  Basic 2D = {:.2} m / Geo 2D dist = {:.2} m / haversine =  {:.2} m",
            dcalc, dcalc2g, dcalc2h
        );

        println!("-----");

        // alt diff in meters
        //
        let drone_alt = drone.alt - home.alt;

        // We have the 2D distance on one side and the elevation relative to home on the other
        // calculate √(x^2 + y^2 + z^2)
        //
        let dist3d = (drone_alt.powi(2) + dcalc.powi(2)).sqrt();
        let dist3dg = (drone_alt.powi(2) + dcalc2g.powi(2)).sqrt();

        println!(
            "  Dist(drone, home) = 3D=({:.2}) or 3Dgeo=({:.2})",
            dist3d, dist3dg
        );
        println!("=====");

        Ok(())
    }
}

mod gravis {
    use rust_3d::Point3D;

    use crate::Point;

    fn earth_radius(lat: f64) -> f64 {
        const EQR: f64 = 6378137.0;
        const POLR: f64 = 6356752.3;

        let t1 = EQR.powi(2) * lat.cos();
        let t2 = POLR.powi(2) * lat.sin();
        let t3 = EQR * lat.cos();
        let t4 = POLR * lat.sin();
        let res = ((t1.powi(2) + t2.powi(2)) / (t3.powi(2) + t4.powi(2))).sqrt();
        res
    }

    fn geocentric_latitude(lat: f64) -> f64 {
        let e2 = 0.00669437999014;
        let res = ((1.0 - e2) * lat.tan()).atan();
        res
    }

    fn location_to_point(pt: Point) -> eyre::Result<Point3D> {
        let lat = pt.lat * std::f64::consts::PI / 180.;
        let lon = pt.lon * std::f64::consts::PI / 180.;
        let radius = earth_radius(lat);
        let clat = geocentric_latitude(lat);

        let nx = lat.cos() * lon.cos();
        let ny = lat.cos() * lon.sin();
        let nz = lat.sin();

        let x = radius * clat.cos() * lon.cos();
        let y = radius * clat.cos() * lon.sin();
        let z = radius * clat.sin();

        let x = x + (pt.alt * nx);
        let y = y + (pt.alt * ny);
        let z = z + (pt.alt * nz);
        Ok(Point3D::new(x, y, z))
    }

    pub fn distances(p1: Point, p2: Point) -> eyre::Result<()> {
        println!("===== gravis =====");

        let drone = location_to_point(p1)?;
        println!("  calc drone = {}", drone);
        let home = location_to_point(p2)?;
        println!("  calc home = {}", home);

        // Regular trig. distances
        //
        let dist2d = ((drone.x - home.x).powi(2) + (drone.y - home.y).powi(2)).sqrt();
        let dist3d = (dist2d.powi(2) + (drone.z - home.z).powi(2)).sqrt();

        println!("----- dist -----");
        println!("  Dist 2D = {:.2}", dist2d);
        println!("  Dist 3D = {:.2}", dist3d);

        Ok(())
    }
}

fn for_points(p1: Point, p2: Point) {
    let _ = roberto::distances(p1, p2);
    let _ = gravis::distances(p1, p2);

    println!("\n******\n");
}

fn main() {
    let _ = roberto::basic();

    let p1 = Point::new(35.3524, 135.0302, 100.);
    let p2 = Point::new(35.3532, 135.0305, 500.);

    for_points(p1, p2);

    let drone = Point::new(48.670105, 2.373384, 190.);
    let home = Point::new(48.66939, 2.369467, 115.);

    for_points(drone, home);

    let nyc = Point::new(40.7128, -74.006, 0.);
    let lon = Point::new(51.5074, -0.1278, 0.);

    for_points(nyc, lon);

    let p1 = Point::new(49.607872, 6.127652, 333.75600000000003);
    let p2 = Point::new(49.6001510620117, 6.14083766937256, 625.);

    for_points(p1, p2);
}
