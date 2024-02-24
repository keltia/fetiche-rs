use chrono::{DateTime, Utc};
use rust_3d::Point3D;

pub use location::*;

mod location;

/// What we read as plane positions
///
#[derive(Debug)]
pub struct Position {
    pub time: DateTime<Utc>,
    pub lat: f64,
    pub lon: f64,
    pub alt: f64,
}

/// Drones journeys
///
#[derive(Debug)]
pub struct Journey {
    pub id: u32,
    pub traj: Vec<Position>,
}

impl From<Position> for Point3D {
    fn from(value: Position) -> Self {
        Point3D::new(value.lon, value.lat, value.alt)
    }
}
