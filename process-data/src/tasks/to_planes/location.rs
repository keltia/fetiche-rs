//! Location related module
//!
//! v1: basic format, only Lat, Lon
//! v2: added [Plus Code](https://plus.codes/)
//! v3: added [GeoHash](https://en.wikipedia.org/wiki/Geohash)
//!
use std::collections::BTreeMap;
use std::fs;

use eyre::{eyre, Result};
use serde::Deserialize;
use tracing::trace;

/// one degree is circumference of earth / 360°, convert into nautical miles
const ONE_DEG_NM: f64 = (40_000. / 1.852) / 360.;

/// Actual location
///
#[derive(Debug, Deserialize)]
pub struct Location {
    /// Plus code encoded location
    pub code: String,
    /// GeoHash string
    pub hash: Option<String>,
    /// Latitude
    pub lat: f64,
    /// Longitude
    pub lon: f64,
}

#[derive(Debug)]
pub struct BB {
    /// Longitude - X0
    pub min_lon: f64,
    /// Latitude - Y0
    pub min_lat: f64,
    /// Longitude - X1
    pub max_lon: f64,
    /// Latitude - Y1
    pub max_lat: f64,
}

impl BB {
    /// Take a location and create a bounding box of `dist` nautical miles away
    ///
    /// So from (lat, lon) we generate the following bounding box:
    /// (lat - dist, lon - dist, lat + dist, lon + dist)
    ///
    #[tracing::instrument]
    pub fn from_location(value: &Location, dist: u32) -> Self {
        // How many degree do we want?
        //
        let dist = dist as f64 / ONE_DEG_NM;

        // Calculate the four corners
        //
        let (min_lat, max_lat) = (value.lat - dist, value.lat + dist);
        let (min_lon, max_lon) = (value.lon - dist, value.lon + dist);

        Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        }
    }
}

/// Current location file version
const LOCATION_FILE_VER: usize = 3;

/// On-disk structure for the locations file
///
#[derive(Debug, Deserialize)]
struct LocationsFile {
    /// Version number for safety
    pub version: usize,
    /// List of locations
    pub location: BTreeMap<String, Location>,
}

/// Load all locations
///
#[tracing::instrument]
pub fn load_locations(fname: Option<String>) -> Result<BTreeMap<String, Location>> {
    trace!("enter");

    // Load from file if specified
    //
    let data = if let Some(fname) = fname {
        fs::read_to_string(fname)?
    } else {
        include_str!("locations.hcl").to_owned()
    };

    let loc: LocationsFile = hcl::from_str(&data)?;
    if loc.version != LOCATION_FILE_VER {
        return Err(eyre!("Bad locations file version, aborting…"));
    }
    Ok(loc.location)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bb_from_location_belfast() -> Result<()> {
        let loc = Location {
            code: "9C6MMRX2+X2".to_string(),
            hash: Some("gcex4vv69".to_string()),
            lat: 54.7,
            lon: -6.2,
        };

        let bb = BB::from_location(&loc, 25);
        assert_eq!(-6.616699695587158, bb.min_lon);
        assert_eq!(54.283302307128906, bb.min_lat);
        assert_eq!(-5.783299922943115, bb.max_lon);
        assert_eq!(55.11669921875, bb.max_lat);
        Ok(())
    }

    #[test]
    fn test_bb_from_location_bxl() -> Result<()> {
        let loc = Location {
            code: "9F26RC22+22".to_string(),
            hash: Some("u150upggr".to_string()),
            lat: 50.8,
            lon: 4.4,
        };

        let bb = BB::from_location(&loc, 25);
        assert_eq!(3.983299970626831, bb.min_lon);
        assert_eq!(50.38330078125, bb.min_lat);
        assert_eq!(4.816699981689453, bb.max_lon);
        assert_eq!(51.216697692871094, bb.max_lat);
        Ok(())
    }
}
