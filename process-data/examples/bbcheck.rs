//!  Create 2D and 3D bounding boxes for a given point and check whether another point
//! is inside or outside our BB.
//!

use rust_3d::{BoundingBox2D, BoundingBox3D, FilterOutlier3D, Point2D, Point3D, Positive};

#[derive(Debug)]
struct Site {
    id: u32,
    name: String,
    code: String,
    lat: f64,
    lon: f64,
}

impl Site {
    // Fun fact, in 2D & 3D geom, x = longitude and y = latitude
    //
    fn get_bb_2d(&self, dist: f64) -> rust_3d::Result<BoundingBox2D> {
        let dist = dist / ONE_DEG_NM;

        let xx = Point2D::new(self.lon - dist, self.lat - dist);
        let yy = Point2D::new(self.lon + dist, self.lat + dist);
        BoundingBox2D::new(&xx, &yy)
    }

    fn get_bb_3d(&self, dist: f64, alt: Option<f64>) -> rust_3d::Result<BoundingBox3D> {
        let alt = alt.unwrap_or(dist);
        let dist = dist / ONE_DEG_NM;

        let xx = Point3D::new(self.lon - dist, self.lat - dist, 0.);
        let yy = Point3D::new(self.lon + dist, self.lat + dist, alt);
        BoundingBox3D::new(&xx, &yy)
    }
}

// 1° in nm
//
const ONE_DEG_NM: f64 = (40_000. / 1.852) / 360.;
const DEF_DIST: f64 = 50.0;

fn main() -> rust_3d::Result<()> {
    let lux = Site {
        id: 1,
        name: "lux".to_string(),
        code: "8FW4H8XX+XX".to_string(),
        lat: 49.6,
        lon: 6.2,
    };

    let bb2 = lux.get_bb_2d(DEF_DIST)?;
    let bb3 = lux.get_bb_3d(DEF_DIST, None)?;
    println!("2D BB = {:?}", bb2);
    println!("3D BB = {:?}", bb3);

    let inside = Point3D::new(6., 50., 25.);

    if bb3.contains(&inside) {
        println!("{:?} is indeed inside {:?}", inside, bb3);
    } else {
        println!("{:?} is NOT inside {:?}", inside, bb3);
    }

    let hundred = 100. / ONE_DEG_NM;
    let mut outside = Point3D::from(inside);

    // Move ourselves
    //
    outside.x += hundred;
    outside.y += hundred / 2.;

    if bb3.contains(&outside) {
        println!("{:?} is indeed inside {:?}", outside, bb3);
    } else {
        println!("{:?} is NOT inside {:?}", outside, bb3);
    }

    let ray = Positive::new(DEF_DIST / ONE_DEG_NM)?;
    let filter = FilterOutlier3D::new(&Point3D::new(lux.lon, lux.lat, 0.), ray, 1)?;

    Ok(())
}
