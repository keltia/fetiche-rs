use geo::{GeodesicDistance, HaversineDistance};
use rust_3d::{IsTransFormableTo2D, Point3D};

mod roberto {
    use geo::{point, GeodesicDistance, HaversineDistance};
    use rust_3d::{dist_2d, dist_3d, IsTransFormableTo2D, Point2D, Point3D};

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

    pub fn method_1(drone: &Point3D, home: &Point3D) -> rust_3d::Result<()> {
        println!("===== roberto =====");
        println!("Geo & 3D stuff in real:");

        // Real lat/lon
        //
        println!("  A={:?}", drone);
        println!("  B={:?}", home);

        println!("-----");

        // Basic trig.
        //
        let d2calc = (drone.x - home.x).powi(2) + (drone.y - home.y).powi(2);
        let dcalc = d2calc.sqrt() * R;

        // Geo stuff, distances are in meters
        //
        let drone2 = point!(x: drone.x, y: drone.y);
        let home2 = point!(x: home.x, y: home.y);
        let dcalc2g = drone2.geodesic_distance(&home2);
        let dcalc2h = drone2.haversine_distance(&home2);
        println!(
            "  Basic 2D = {:.2} m / Geo 2D dist = {:.2} m / haversine =  {:.2} m",
            dcalc, dcalc2g, dcalc2h
        );

        println!("-----");

        // alt diff in meters
        //
        let drone_alt = drone.z - home.z;

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

    struct Point {
        pub lat: f64,
        pub lon: f64,
        pub alt: f64,
    }

    fn earth_radius(lat: f64) -> f64 {
        const eqR: f64 = 6378137.0;
        const polR: f64 = 6356752.3;

        let t1 = eqR.powi(2) * lat.cos();
        let t2 = polR.powi(2) * lat.sin();
        let t3 = eqR * lat.cos();
        let t4 = polR * lat.sin();
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
        println!("radius={}", radius);
        let clat = geocentric_latitude(lat);
        println!("geocentric_lat={}", clat);

        let nx = lat.cos() * lon.cos();
        let ny = lat.cos() * lon.sin();
        let nz = lat.sin();

        let x = radius * clat.cos() * lon.cos();
        let y = radius * clat.cos() * lon.sin();
        let z = radius * clat.sin();

        let x = x + (pt.alt * nx);
        let y = y + (pt.alt * ny);
        let y = z + (pt.alt * nz);
        Ok(Point3D::new(x, y, z))
    }

    pub fn method_2(p1: &Point3D, p2: &Point3D) -> eyre::Result<()> {
        println!("===== gravis =====");
        println!("Geo & 3D stuff in real:");

        let pt1 = Point {
            lon: p1.x,
            lat: p1.y,
            alt: p1.z,
        };
        let pt2 = Point {
            lon: p2.x,
            lat: p2.y,
            alt: p2.z,
        };

        let drone = location_to_point(pt1)?;
        println!("calc drone = {}", drone);
        let home = location_to_point(pt2)?;
        println!("calc home = {}", home);

        // Regular trig. distances
        //
        let dist2d = ((drone.x - home.x).powi(2) + (drone.y - home.y).powi(2)).sqrt();
        let dist3d = (dist2d.powi(2) + (drone.z - home.z).powi(2)).sqrt();

        println!("----- dist -----");
        println!(" Dist 2D = {:.2}", dist2d);
        println!(" Dist 3D = {:.2}", dist3d);

        Ok(())
    }
}

fn main() {
    let _ = roberto::basic();

    let p1 = Point3D::new(35.3524, 135.0302, 100.);
    let p2 = Point3D::new(35.3532, 135.0305, 500.);

    let _ = roberto::method_1(&p1, &p2);
    let _ = gravis::method_2(&p1, &p2);

    println!("\n******\n");
    let drone = Point3D::new(2.373384, 48.670105, 190.);
    let home = Point3D::new(2.369467, 48.66939, 115.);

    let _ = roberto::method_1(&drone, &home);
    let _ = gravis::method_2(&drone, &home);

    println!("\n******\n");
    let nyc = Point3D::new(-74.006, 40.7128, 0.);
    let lon = Point3D::new(-0.1278, 51.5074, 0.);

    let _ = roberto::method_1(&nyc, &lon);
    let _ = gravis::method_2(&nyc, &lon);

    println!("\n******\n");
    let p1 = Point3D::new(49.607872, 6.127652, 333.75600000000003);
    let p2 = Point3D::new(49.6001510620117, 6.14083766937256, 625.);

    let _ = roberto::method_1(&p1, &p2);
    let _ = gravis::method_2(&p1, &p2);
}
