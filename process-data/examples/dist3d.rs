use rust_3d::{dist_2d, dist_3d, IsTransFormableTo2D, Point2D, Point3D};

fn distances(a: &Point3D, b: &Point3D) {
    let a2: Point2D = a.transform_to_2d();
    let b2: Point2D = b.transform_to_2d();

    let d2 = dist_2d(&a2, &b2);
    let d3 = dist_3d(&a, &b);

    println!("Dist(A,B) = 2D=({}) or 3D=({})", d2, d3);
}

fn main() -> rust_3d::Result<()> {
    let a3 = Point3D::new(0., 0., 0.);
    let b3 = Point3D::new(1., 1., 1.);
    let c3 = Point3D::new(1., 1., -1.);

    distances(&a3, &b3);
    distances(&a3, &c3);
    distances(&b3, &c3);

    Ok(())
}
