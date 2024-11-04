//! Module for exporting encounters into KML files.
//!

use chrono::{DateTime, Utc};
use clap::Parser;
use eyre::{format_err, Result};
use klickhouse::{QueryBuilder, Row};
use kml::Kml::Document;
use kml::{Kml, KmlDocument, KmlVersion};
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, trace};

use crate::cmds::Format;
use crate::config::Context;

#[derive(Debug, Parser)]
pub struct ExpEncounterOpts {
    /// Export every encounter in its own file.
    #[clap(short = 'A', long)]
    all: bool,
    /// Export that Encounter ID
    #[clap(long)]
    id: Option<String>,
    /// Format (default is KML)
    #[clap(short = 'F', long, default_value = "kml", value_parser)]
    format: Format,
    /// Output file or directory.
    #[clap(short = 'o', long)]
    output: Option<PathBuf>,
}

/// Export one or all existing encounters as KML files into a single file/directory
///
pub async fn export_encounters(ctx: &Context, opts: &ExpEncounterOpts) -> Result<()> {
    // Check arguments
    //
    let all = opts.all;
    let id = &opts.id.clone();
    let output = &opts.output.clone();

    // Can't have -A and an ID
    //
    if all && id.is_some() {
        return Err(format_err!("Either -A or --id, not both!"));
    }

    if all {
        // We need output to be specified and a directory
        //
        if let Some(output) = output {
            if !output.is_dir() {
                return Err(format_err!(
                    "output path {:?} given, expected a directory",
                    output
                ));
            }

            let n = export_all_encounter(ctx).await?;
            eprintln!("Exported {n} files within {output:?}");
        } else {
            return Err(format_err!("No output path specified.'"));
        }
    } else {
        // A single en_id is requested
        //
        let en_id = match opts.id.clone() {
            Some(id) => id,
            None => return Err(format_err!("No encounter ID given")),
        };

        let kml = export_one_encounter(ctx, &en_id).await?;

        let output = if output.is_none() {
            PathBuf::from(format!("{}.kml", en_id))
        } else {
            output.clone().unwrap()
        };
        let _ = fs::write(&output, kml).await?;
        eprintln!("Exported {en_id} in {output:?}");
    }
    Ok(())
}

#[derive(Clone, Debug, Row, Serialize)]
struct DataPoint {
    timestamp: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    altitude: f64,
}

/// Export one single encounter
///
async fn export_one_encounter(ctx: &Context, id: &str) -> Result<String> {
    let client = ctx.db().await;

    let re = Regex::new(r##"^(?<name>[A-Z]{3})-(?<date>\d{8})-(?<journey>\d+)-(\d+)$"##)?;

    let (name, date, journey) = if let Some(caps) = re.captures(id) {
        let date = &caps["date"];

        let re = Regex::new(r##"^(?<year>\d{4})(?<month>\d{2})(?<day>\d{2})$"##)?;
        if re.captures(date).is_none() {
            return Err(format_err!("Bad date format in {id}"));
        }
        (
            &caps["name"].to_string(),
            date.to_string(),
            caps["journey"].parse::<i32>()?,
        )
    } else {
        return Err(format_err!("bad en_id"));
    };
    debug!("name: {}, date: {}, journey: {}", name, date, journey);

    #[derive(Clone, Debug, Row)]
    struct Encounter {
        en_id: String,
        journey: i32,
        drone_id: String,
        prox_id: String,
        prox_callsign: String,
    }

    // Fetch the drone & airplane IDs
    //
    let rp = r##"
SELECT
  en_id, journey, drone_id, prox_callsign, prox_id
FROM airplane_prox
WHERE en_id = $1
    "##;
    let q = QueryBuilder::new(rp).arg(id);
    let res = client.query_one::<Encounter>(q).await?;

    assert_eq!(res.en_id, id);
    assert_eq!(res.journey, journey);

    let drone_id = res.drone_id.clone();

    // ICAO string is unique, whereas callsign can change
    //
    let prox_id = res.prox_id.clone();
    let prox_callsign = res.prox_callsign.clone();

    // Fetch drone points
    //
    let rpp = r##"
SELECT
  toDateTime(timestamp) as timestamp,
  latitude,
  longitude,
  toFloat64(altitude) AS altitude
FROM drones
WHERE
journey = $1 AND
ident = $2
ORDER BY timestamp
    "##;

    let q = QueryBuilder::new(rpp).arg(journey).arg(&drone_id);
    let drone = client.query_collect::<DataPoint>(q).await?;
    trace!("Found {} drone points for en_id {}", drone.len(), drone_id);

    dbg!(&drone);

    // Extract first and last timestamp to have a suitable interval for plane points.
    //
    if drone.len() <= 1 {
        return Err(format_err!("no drones found"));
    }

    let first = drone.first().unwrap().timestamp;
    let last = drone.last().unwrap().timestamp;

    // Fetch plane points
    //
    let rdp = r##"
SELECT
  time,
  prox_lat AS latitude,
  prox_lon AS longitude,
  prox_alt AS altitude
FROM airplanes
WHERE
  prox_id = $1 AND
  time BETWEEN $2 AND $3
ORDER BY time
    "##;

    let q = QueryBuilder::new(rdp).arg(&prox_id).arg(first).arg(last);
    let plane = client.query_collect::<DataPoint>(q).await?;
    trace!("Found {} plane points for id {}", plane.len(), prox_id);

    dbg!(&plane);

    // Define our styles
    //
    let d_style = create::make_style("droneStyle", "00ff0000", 4.);
    let p_style = create::make_style("planeStyle", "0000ff00", 4.);

    // Create PlaceMark for each trajectory
    //
    let drone = create::from_traj_to_placemark(&drone_id, &drone, "droneStyle")?;
    let plane = create::from_traj_to_placemark(&prox_callsign, &plane, "planeStyle")?;

    let doc = Document {
        attrs: [("name".into(), format!("{id}.kml"))].into(),
        elements: vec![d_style, p_style, drone, plane],
    };

    let kml = KmlDocument {
        version: KmlVersion::V23,
        elements: vec![doc.into()],
        attrs: HashMap::new(),
    };

    let kml = Kml::KmlDocument(kml.into());

    Ok(kml.to_string())
}

/// Export all encounters
///
/// TODO: write it.
///
async fn export_all_encounter(ctx: &Context) -> Result<usize> {
    let _client = ctx.db().await;

    eprintln!("Exporting all encounters... DUMMY");
    Ok(0)
}

// -----

/// Small internal module to manipulate XML data types
///
mod create {
    use super::DataPoint;
    use kml::{
        types::{AltitudeMode, Coord, Geometry, LineString, LineStyle, Placemark, Style},
        Kml,
    };
    use std::collections::HashMap;

    /// Generate a `LineString` given a list of (x,y,z) points.
    ///
    fn from_points_to_ls(points: &Vec<DataPoint>) -> eyre::Result<LineString> {
        let coords = points
            .into_iter()
            .map(|p| {
                Coord::new(
                    p.longitude as f64,
                    p.latitude as f64,
                    Some(p.altitude as f64),
                )
            })
            .collect::<Vec<_>>();

        Ok(LineString {
            tessellate: false,
            extrude: true,
            altitude_mode: AltitudeMode::Absolute,
            coords: coords.into(),
            ..Default::default()
        })
    }

    /// Create a `Style`  entry for a `Placemark`
    ///
    pub(crate) fn make_style(name: &str, colour: &str, size: f64) -> Kml {
        Kml::Style(Style {
            id: Some(name.into()),
            line: LineStyle {
                color: colour.into(),
                width: size,
                ..Default::default()
            }
                .into(),
            ..Default::default()
        })
    }

    /// Create a `Placemark` given a name (like drone or plane ID) and its trajectory using the
    /// requested style.
    ///
    pub(crate) fn from_traj_to_placemark(
        name: &str,
        points: &Vec<DataPoint>,
        style: &str,
    ) -> eyre::Result<Kml> {
        let ls = from_points_to_ls(points)?;
        let style_url = format!("#{style}");
        Ok(Kml::Placemark(Placemark {
            name: Some(name.into()),
            geometry: Some(Geometry::LineString(ls.into())),
            attrs: HashMap::from([("styleUrl".into(), style_url)]),
            ..Default::default()
        }))
    }
}
