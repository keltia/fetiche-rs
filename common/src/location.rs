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
use serde::Deserialize;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::trace;

/// one degree is circumference of earth / 360°, convert into nautical miles
const ONE_DEG_NM: f64 = (40_000. / 1.852) / 360.;

/// Actual location (this is the same schema as `sites.csv`).
///
#[derive(Clone, Debug, Deserialize)]
pub struct Location {
    /// Site id
    pub id: u32,
    /// Short name
    pub name: String,
    /// Plus code encoded location
    pub code: String,
    /// geoHash
    pub hash: Option<String>,
    /// More descriptive name
    pub basename: String,
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Reference altitude
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
        Self::from_lat_lon(value.latitude, value.longitude, dist)
    }

    /// Take a lat lot tuple and create a bounding box of `dist` nautical miles away
    ///
    /// So from (lat, lon) we generate the following bounding box:
    /// (lat - dist, lon - dist, lat + dist, lon + dist)
    ///
    /// NOTE: `dist` is in Nautical Miles
    ///
    #[tracing::instrument]
    pub fn from_lat_lon(lat: f64, lon: f64, dist: u32) -> Self {
        let dist = dist as f64 / ONE_DEG_NM;

        // Calculate the four corners
        //
        let (min_lat, max_lat) = (lat - dist, lat + dist);
        let (min_lon, max_lon) = (lon - dist, lon + dist);

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
        include_str!("sites.csv").to_owned()
    };

    let mut list = BTreeMap::<String, Location>::new();
    let mut rdr = csv::Reader::from_reader(data.as_bytes());
    rdr.deserialize().for_each(|line| {
        let rec: Location = line.unwrap();
        list.insert(rec.name.clone(), rec);
    });

    Ok(list)
}

/// List loaded locations
///
#[tracing::instrument]
pub fn list_locations(data: &BTreeMap<String, Location>, dist: u32) -> Result<String> {
    trace!("enter");
    let header = vec![
        "Location",
        "Plus Code",
        "Basename",
        "Lat/Lon",
        "Altitude",
        "Polygon",
    ];

    let mut builder = Builder::default();
    builder.push_record(header);

    data.keys().for_each(|name| {
        let mut row = vec![];

        let loc = data.get(name).unwrap();
        let code = loc.code.clone();
        let ref_alt = format!("{}", loc.ref_altitude);
        let basename = loc.basename.clone();
        let poly = BB::from_location(loc, dist);
        let point = format!("{:.5}, {:.5}", loc.latitude, loc.longitude);
        let poly = format!(
            "{:.2}, {:.2}, {:.2}, {:.2}",
            poly.min_lat, poly.min_lon, poly.max_lat, poly.max_lon
        );
        row.push(name);
        row.push(&code);
        row.push(&basename);
        row.push(&point);
        row.push(&ref_alt);
        row.push(&poly);
        builder.push_record(row);
    });

    let allf = builder.build().with(Style::modern()).to_string();
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
        assert_eq!(shorten(-6.616699695587158), shorten(bb.min_lon));
        assert_eq!(shorten(54.283302307128906), shorten(bb.min_lat));
        assert_eq!(shorten(-5.783299922943115), shorten(bb.max_lon));
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
        assert_eq!(shorten(3.983299970626831), shorten(bb.min_lon));
        assert_eq!(shorten(50.38330078125), shorten(bb.min_lat));
        assert_eq!(shorten(4.816699981689453), shorten(bb.max_lon));
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
        assert_eq!(shorten(3.983299970626831), shorten(x0));
        assert_eq!(shorten(50.38330078125), shorten(y0));
        assert_eq!(shorten(4.816699981689453), shorten(x1));
        assert_eq!(shorten(51.216697692871094), shorten(y1));
        Ok(())
    }
}
