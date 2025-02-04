use egm2008::geoid_height;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Locations {
    version: usize,
    location: HashMap<String, Location>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Location {
    /// Plus code encoded location
    pub code: String,
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lon: f64,
    /// Reference altitude
    pub alt: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_str = include_str!("../src/locations.hcl");

    let list: Locations = hcl::from_str(file_str)?;
    list.location.iter().for_each(|(n, l)| {
        let diff_h = geoid_height(l.lat as f32, l.lon as f32).unwrap();
        println!("{}: stored altitude={:3} EGM2008={:.5} goemetric altitude={:.5}", n, l.alt, diff_h, l.alt as f32 + diff_h);
    });

    Ok(())
}