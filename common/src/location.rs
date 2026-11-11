//! Location related module
//!
//! v1: basic format, only Lat, Lon
//! v2: added [Plus Code](https://plus.codes/)
//! v3: added [GeoHash](https://en.wikipedia.org/wiki/Geohash)
//!
//! NOTE: no more history as data is now loaded from the `sites.csv` file.
//!
use std::collections::BTreeMap;
use std::fs;

use eyre::Result;
use serde::{Deserialize, Serialize};
use tabled::settings::Style;
use tabled::{Table, Tabled};
use tracing::trace;

/// one degree is circumference of earth / 360°, convert into nautical miles
const ONE_DEG_NM: f64 = (40_000. / 1.852) / 360.;

fn display_option(o: &Option<String>) -> String {
    match o {
        Some(s) => s.clone(),
        None => String::new(),
    }
}

/// Actual location (this is the same schema as `sites.csv`).
///
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Tabled)]
pub struct Location {
    /// Site id
    #[tabled(rename = "ID")]
    pub id: u32,
    /// Short name
    #[tabled(rename = "Location")]
    pub name: String,
    /// Plus code encoded location
    #[tabled(rename = "Plus Code")]
    pub code: String,
    /// geoHash
    #[tabled(rename = "GeoHash", display("display_option"))]
    pub hash: Option<String>,
    /// More descriptive name
    #[tabled(rename = "Basename")]
    pub basename: String,
    /// Latitude
    #[tabled(rename = "Latitude")]
    pub latitude: f64,
    /// Longitude
    #[tabled(rename = "Longitude")]
    pub longitude: f64,
    /// Reference altitude
    #[tabled(rename = "Altitude (m)")]
    pub ref_altitude: f64,
}

impl Default for Location {
    fn default() -> Self {
        Location {
            id: 0,
            name: String::new(),
            code: String::new(),
            hash: Some(String::new()),
            basename: String::new(),
            latitude: 0.,
            longitude: 0.,
            ref_altitude: 0.,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Take a location and create a bounding box of `dist` nautical miles away.
    ///
    /// Longitude spans are scaled by the cosine of the latitude to ensure consistent physical distance.
    #[tracing::instrument]
    pub fn from_location(value: &Location, dist: u32) -> Self {
        Self::from_lat_lon(value.latitude, value.longitude, dist)
    }

    /// Take a (lat, lon) tuple and create a bounding box of `dist` nautical miles away.
    ///
    /// Longitude span is compensated for latitude curvature:
    /// `lon_delta = (dist / ONE_DEG_NM) / cos(lat)`.
    ///
    /// NOTE: `dist` is in Nautical Miles.
    #[tracing::instrument]
    pub fn from_lat_lon(lat: f64, lon: f64, dist: u32) -> Self {
        let lat_delta = dist as f64 / ONE_DEG_NM;
        let cos_lat = lat.to_radians().cos().abs().max(1e-6);
        let lon_delta = lat_delta / cos_lat;

        // Calculate the four corners
        //
        let (min_lat, max_lat) = (lat - lat_delta, lat + lat_delta);
        let (min_lon, max_lon) = (lon - lon_delta, lon + lon_delta);

        Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        }
    }

    /// Generate an array with the four points in a BB
    ///
    #[tracing::instrument]
    pub fn to_polygon(&self) -> Result<[(f64, f64); 4]> {
        Ok([
            (self.min_lon, self.min_lat),
            (self.min_lon, self.max_lat),
            (self.max_lon, self.max_lat),
            (self.max_lon, self.min_lat),
        ])
    }
}

/// Load all locations from the `sites.csv` file instead of a separate `locations.hcl`.
///
#[tracing::instrument]
pub fn load_locations(fname: Option<String>) -> Result<BTreeMap<String, Location>> {
    trace!("enter");

    // Load from file if specified
    //
    let data = if let Some(fname) = fname {
        fs::read_to_string(fname)?
    } else {
        include_str!("../../data/sites.csv").to_owned()
    };

    let mut list = BTreeMap::<String, Location>::new();
    let mut rdr = csv::Reader::from_reader(data.as_bytes());
    for line in rdr.deserialize() {
        let rec: Location = line?;
        list.insert(rec.name.clone(), rec);
    }

    Ok(list)
}

#[derive(Tabled)]
struct LocationRow<'a> {
    #[tabled(rename = "Location")]
    name: &'a str,
    #[tabled(rename = "Plus Code")]
    code: &'a str,
    #[tabled(rename = "Basename")]
    basename: &'a str,
    #[tabled(rename = "Lat/Lon")]
    point: String,
    #[tabled(rename = "Altitude")]
    ref_alt: f64,
    #[tabled(rename = "Polygon")]
    poly: String,
}

/// List loaded locations
///
#[tracing::instrument]
pub fn list_locations(data: &BTreeMap<String, Location>, dist: u32) -> Result<String> {
    trace!("enter");
    let rows: Vec<_> = data
        .values()
        .map(|loc| {
            let poly = BB::from_location(loc, dist);
            let point = format!("{:.5}, {:.5}", loc.latitude, loc.longitude);
            let poly_str = format!(
                "{:.2}, {:.2}, {:.2}, {:.2}",
                poly.min_lat, poly.min_lon, poly.max_lat, poly.max_lon
            );
            LocationRow {
                name: &loc.name,
                code: &loc.code,
                basename: &loc.basename,
                point,
                ref_alt: loc.ref_altitude,
                poly: poly_str,
            }
        })
        .collect();

    let allf = Table::new(rows).with(Style::sharp()).to_string();
    Ok(format!("List all locations ({dist} nm):\n{allf}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    #[inline]
    fn shorten(v: f64) -> String {
        format!("{:.3}", v)
    }

    #[test_pretty_log::test]
    fn test_bb_from_location_belfast() -> Result<()> {
        info!("bel");
        let loc = Location {
            code: "9C6MMRX2+X2".to_string(),
            hash: Some("gcex4vv69".to_string()),
            latitude: 54.7,
            longitude: -6.2,
            ..Default::default()
        };

        let bb = BB::from_location(&loc, 25);
        assert_eq!(shorten(-6.921), shorten(bb.min_lon));
        assert_eq!(shorten(54.283302307128906), shorten(bb.min_lat));
        assert_eq!(shorten(-5.4788), shorten(bb.max_lon));
        assert_eq!(shorten(55.11669921875), shorten(bb.max_lat));
        Ok(())
    }

    #[test_pretty_log::test]
    fn test_bb_from_location_bxl() -> Result<()> {
        info!("bxl");
        let loc = Location {
            code: "9F26RC22+22".to_string(),
            hash: Some("u150upggr".to_string()),
            latitude: 50.8,
            longitude: 4.4,
            ..Default::default()
        };

        let bb = BB::from_location(&loc, 25);
        assert_eq!(shorten(3.7406883419544416), shorten(bb.min_lon));
        assert_eq!(shorten(50.38330078125), shorten(bb.min_lat));
        assert_eq!(shorten(5.059311658045558), shorten(bb.max_lon));
        assert_eq!(shorten(51.216697692871094), shorten(bb.max_lat));
        Ok(())
    }

    #[test_pretty_log::test]
    fn test_to_polygon() -> Result<()> {
        let loc = Location {
            code: "9F26RC22+22".to_string(),
            hash: Some("u150upggr".to_string()),
            latitude: 50.8,
            longitude: 4.4,
            ..Default::default()
        };

        let abb = BB::from_location(&loc, 25).to_polygon();
        assert!(abb.is_ok());
        let abb = abb.unwrap();
        let x0 = abb[0].0;
        let x1 = abb[2].0;
        let y0 = abb[0].1;
        let y1 = abb[2].1;
        assert_eq!(shorten(3.7406883419544416), shorten(x0));
        assert_eq!(shorten(50.38330078125), shorten(y0));
        assert_eq!(shorten(5.059311658045558), shorten(x1));
        assert_eq!(shorten(51.216697692871094), shorten(y1));
        Ok(())
    }

    #[test]
    fn test_bb_equator_scaling() {
        let bb = BB::from_lat_lon(0.0, 10.0, 60);
        let lat_delta = bb.max_lat - 0.0;
        let lon_delta = bb.max_lon - 10.0;
        // At equator, cos(0) == 1.0, so lon_delta == lat_delta
        assert!((lat_delta - lon_delta).abs() < 1e-9);
    }

    #[test]
    fn test_bb_high_latitude_scaling() {
        let bb = BB::from_lat_lon(60.0, 10.0, 60);
        let lat_delta = bb.max_lat - 60.0;
        let lon_delta = bb.max_lon - 10.0;
        // At 60 deg lat, cos(60) == 0.5, so lon_delta == 2.0 * lat_delta
        assert!((lon_delta - 2.0 * lat_delta).abs() < 1e-4);
    }

    #[test]
    fn test_bb_southern_hemisphere_scaling() {
        let bb_north = BB::from_lat_lon(45.0, 0.0, 30);
        let bb_south = BB::from_lat_lon(-45.0, 0.0, 30);
        // Symmetry between North and South latitudes
        assert_eq!(shorten(bb_north.max_lon), shorten(bb_south.max_lon));
        assert_eq!(shorten(bb_north.min_lon), shorten(bb_south.min_lon));
    }

    #[test]
    fn test_bb_pole_no_panic() {
        let bb = BB::from_lat_lon(90.0, 0.0, 30);
        assert!(!bb.min_lon.is_nan() && !bb.min_lon.is_infinite());
        assert!(!bb.max_lon.is_nan() && !bb.max_lon.is_infinite());
    }

    #[test]
    fn test_load_locations_default() -> Result<()> {
        let locs = load_locations(None)?;
        assert!(!locs.is_empty());
        assert!(locs.contains_key("BEL"));
        assert!(locs.contains_key("BRU"));
        let bel = &locs["BEL"];
        assert_eq!(bel.name, "BEL");
        assert_eq!(bel.basename, "Belfast");
        Ok(())
    }

    #[test]
    fn test_load_locations_custom_file() -> Result<()> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new()?;
        writeln!(
            tmp,
            "\"id\",\"name\",\"code\",\"basename\",\"latitude\",\"longitude\",\"ref_altitude\"\n1,\"TEST\",\"8FW4J82W+2R\",\"TestSite\",48.5,2.3,100"
        )?;
        let path = tmp.path().to_string_lossy().to_string();
        let locs = load_locations(Some(path))?;
        assert_eq!(locs.len(), 1);
        assert!(locs.contains_key("TEST"));
        assert_eq!(locs["TEST"].basename, "TestSite");
        Ok(())
    }

    #[test]
    fn test_load_locations_invalid_csv() -> Result<()> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new()?;
        writeln!(
            tmp,
            "\"id\",\"name\",\"code\",\"basename\",\"latitude\",\"longitude\",\"ref_altitude\"\n1,\"TEST\",\"8FW4J82W+2R\",\"TestSite\",not_a_latitude,2.3,100"
        )?;
        let path = tmp.path().to_string_lossy().to_string();
        let res = load_locations(Some(path));
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn test_load_locations_nonexistent_file() {
        let res = load_locations(Some("non_existent_file_path_12345.csv".to_string()));
        assert!(res.is_err());
    }

    #[test]
    fn test_list_locations_output() -> Result<()> {
        let mut locs = BTreeMap::new();
        locs.insert(
            "TEST".to_string(),
            Location {
                id: 1,
                name: "TEST".to_string(),
                code: "8FW4J82W+2R".to_string(),
                hash: Some("u09t".to_string()),
                basename: "TestSite".to_string(),
                latitude: 48.5,
                longitude: 2.3,
                ref_altitude: 100.0,
            },
        );

        let table = list_locations(&locs, 25)?;
        assert!(table.contains("List all locations (25 nm):"));
        assert!(table.contains("TEST"));
        assert!(table.contains("8FW4J82W+2R"));
        assert!(table.contains("TestSite"));
        assert!(table.contains("48.50000, 2.30000"));
        assert!(table.contains("100"));
        Ok(())
    }

    #[test]
    fn test_list_locations_empty() -> Result<()> {
        let locs = BTreeMap::new();
        let table = list_locations(&locs, 10)?;
        assert!(table.contains("List all locations (10 nm):"));
        assert!(table.contains("Location"));
        Ok(())
    }

    #[test]
    fn test_location_tabled_derive() {
        let loc = Location {
            id: 1,
            name: "TEST".to_string(),
            code: "8FW4J82W+2R".to_string(),
            hash: Some("u09t".to_string()),
            basename: "TestSite".to_string(),
            latitude: 48.5,
            longitude: 2.3,
            ref_altitude: 100.0,
        };
        let table = Table::new([loc]).to_string();
        assert!(table.contains("TEST"));
        assert!(table.contains("8FW4J82W+2R"));
        assert!(table.contains("u09t"));
    }

    #[test]
    #[allow(clippy::clone_on_copy)]
    fn test_bb_traits() {
        let bb = BB {
            min_lon: 1.0,
            min_lat: 2.0,
            max_lon: 3.0,
            max_lat: 4.0,
        };

        // Test Copy, Clone, PartialEq
        let copied = bb;
        let cloned = bb.clone();
        assert_eq!(bb, copied);
        assert_eq!(bb, cloned);

        // Test Serialize, Deserialize
        let serialized = serde_json::to_string(&bb).unwrap();
        let deserialized: BB = serde_json::from_str(&serialized).unwrap();
        assert_eq!(bb, deserialized);
    }
}
