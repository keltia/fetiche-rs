use std::collections::BTreeMap;
use std::fs;

use anyhow::{anyhow, Result};
use serde::Deserialize;

/// one degree is circumference of earth / 360°
const ONE_DEG_NM: f32 = 40_000.0 / 360;
/// Current location file version
const LOCATION_FILE_VER: usize = 1;

/// Take a position and create a bounding box of `dist` nautical miles away
///
#[tracing::instrument]
pub fn generate_bounding_box(lat: f32, lon: f32, dist: u32) -> [f32; 4] {
    // How many degree do we want?
    //
    let dist = dist as f32 / ONE_DEG_NM;

    // Calculate the four corners
    //
    let (min_lat, max_lat) = (lat - dist, lat + dist);
    let (min_lon, max_lon) = (lon - dist, lon + dist);

    [min_lat, max_lat, min_lon, max_lon]
}

/// On-disk structure for the locations file
///
#[derive(Debug, Deserialize)]
struct LocationsFile {
    /// Version number for safety
    pub version: usize,
    /// List of locations
    pub data: BTreeMap<String, Location>,
}

/// Actual location
///
#[derive(Debug, Deserialize)]
pub struct Location {
    /// Latitude
    pub lat: f32,
    /// Longitude
    pub lon: f32,
}

/// Load all locations
///
pub fn load_locations(fname: Option<String>) -> Result<BTreeMap<String, Location>> {
    let data = match fname {
        Some(fname) => fs::read_to_string(&fname),
        None => Some(include_str!("locations.hcl")),
    }?;
    let loc: LocationsFile = hcl::from_str(&data)?;
    if loc.version != LOCATION_FILE_VER {
        return Err(anyhow!("Bad locations file version, aborting…"));
    }
    Ok(loc.data)
}

/// List loaded locations
///
pub fn list_locations(data: &BTreeMap<String, Location>) -> Result<String> {
    Ok(data
        .keys()
        .map(|name| {
            let loc = data.get(name).unwrap();

            format!("Location: {} — {:.2}, {:.2}", name, loc.lat, loc.lon)
        })
        .join("\n"))
}
