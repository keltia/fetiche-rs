use geo::{point, GeodesicDistance, HaversineDistance};
use rust_3d::{dist_2d, dist_3d, IsTransFormableTo2D, Point2D, Point3D};

/// 1 deg = 59.9952 nm or 111.1111 km
const R: f64 = 111_111.11;

/// Basic trig, no geo stuff
///
fn trig_distances(a: &Point3D, b: &Point3D) -> (f64, f64) {
    let a2: Point2D = a.transform_to_2d();
    let b2: Point2D = b.transform_to_2d();

    let d2 = dist_2d(&a2, &b2);
    let d3 = dist_3d(a, b);
    (d2, d3)
}

fn main() -> rust_3d::Result<()> {
    let a3 = Point3D::new(0., 0., 0.);
    let b3 = Point3D::new(1., 1., 1.);
    let c3 = Point3D::new(1., 1., -1.);

    let (ab2, ab3) = trig_distances(&a3, &b3);
    let (ac2, ac3) = trig_distances(&a3, &c3);

    println!("=====");
    println!("Basic 2D/3D:");
    println!("  Dist(A,B) = 2D=({:.2}) or 3D=({:.2})", ab2, ab3);
    println!("  Dist(A,C) = 2D=({:.2}) or 3D=({:.2})", ac2, ac3);

    println!("=====");
    println!("Geo & 3D stuff in real:");

    // Real lat/lon
    //
    let drone = Point3D::new(2.373384, 48.670105, 190. / R);
    let home = Point3D::new(2.369467, 48.66939, 115. / R);

    println!("  A={:?}", drone);
    println!("  B={:?}", home);

    println!("-----");

    // Basic trig.
    //
    let d2calc = (drone.x - home.x).powi(2) + (drone.y - home.y).powi(2);
    let dcalc = d2calc.sqrt() * R;
    println!("  Basic 2D dist = {:.2}", dcalc);

    // Geo stuff
    //
    let drone2 = point!(x: drone.x, y: drone.y);
    let home2 = point!(x: home.x, y: home.y);
    let dcalc2 = drone2.geodesic_distance(&home2);
    let dcalc2h = drone2.haversine_distance(&home2);
    println!(
        "  Geo 2D dist = {:.2} m / haversine =  {:.2} m",
        dcalc2, dcalc2h
    );

    println!("-----");

    // alt diff in meters
    //
    let drone_alt = drone.z - home.z;
    assert!(drone_alt >= 0.);

    // In deg.
    //
    let drone_alt = drone_alt / 111_111.1;

    // We have the 2D distance on one side and the elevation relative to home on the other
    // calculate √(x^2 + y^2 + z^2)
    //
    let tmp = (drone_alt.powi(2) + (dcalc2 / 111_111.1).powi(2)).sqrt() * 111_111.1;

    println!(
        "  Dist(drone, home) = 2D=({:.2}) or 3D=({:.2})",
        dcalc2, tmp
    );
    println!("=====");

    Ok(())
}
