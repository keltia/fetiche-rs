//! Module for exporting encounters into KML files.
//!

use chrono::{DateTime, Utc};
use clap::Parser;
use colorsys::Rgb;
use eyre::{format_err, Result};
use futures::future::join_all;
use itertools::Itertools;
use klickhouse::Row;
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

/// Main struct for data points, both drone and plane
///
#[derive(Clone, Debug, Row, Serialize)]
struct DataPoint {
    timestamp: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
    altitude: f64,
}

/// What we need from the `airplane_prox` table.
///
#[derive(Clone, Debug, Row, Serialize)]
struct Encounter {
    en_id: String,
    journey: i32,
    drone_id: String,
    prox_id: String,
    prox_callsign: String,
}

/// Export one or all existing encounters as KML files into a single file/directory
///
#[tracing::instrument(skip(ctx))]
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
        trace!("Exporting all encounters.");

        // We need output to be specified and a directory
        //
        if let Some(output) = output {
            if !output.is_dir() {
                return Err(format_err!(
                    "output path {:?} given, expected a directory",
                    output
                ));
            }

            let n = export_all_encounter(ctx, output).await?;
            eprintln!("Exported {n} files within {output:?}");
        } else {
            return Err(format_err!("No output path specified.'"));
        }
    } else {
        trace!("Exporting one encounter.");

        // A single en_id is requested
        //
        let en_id = match opts.id.clone() {
            Some(id) => id,
            None => return Err(format_err!("No encounter ID given")),
        };

        let kml = export_one_encounter(ctx, &en_id).await?;

        let output = match output {
            None => PathBuf::from(format!("{}.kml", en_id)),
            Some(output) => {
                if output.is_dir() {
                    PathBuf::from(output).join(PathBuf::from(format!("{}.kml", en_id)))
                } else {
                    output.clone()
                }
            }
        };
        let _ = fs::write(&output, kml).await?;
        eprintln!("Exported {en_id} in {output:?}");
    }
    Ok(())
}

/// Export one single encounter
///
#[tracing::instrument(skip(ctx))]
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

    let res = data::fetch_one_encounter(&client, &id).await?;

    assert_eq!(res.en_id, id);
    assert_eq!(res.journey, journey);

    let drone_id = res.drone_id.clone();

    // ICAO string is unique, whereas callsign can change
    //
    let prox_id = res.prox_id.clone();
    let prox_callsign = res.prox_callsign.clone();

    let drones = data::fetch_drones(&client, journey, &drone_id).await?;
    if drones.len() <= 1 {
        return Err(format_err!("no drones found"));
    }

    // Extract first and last timestamp to have a suitable interval for plane points.
    //
    let first = drones.first().unwrap().timestamp;
    let last = drones.last().unwrap().timestamp;

    // We use `prox_id` because this one does not change whereas callsign can and will
    //
    let planes = data::fetch_planes(&client, &prox_id, first, last).await?;
    if planes.len() <= 1 {
        return Err(format_err!("no planes found"));
    }

    // Define our styles
    //
    let red = Rgb::from((255., 0., 0., 1.0));
    let green = Rgb::from((0., 255., 0., 1.0));

    // We need the alpha channel for some reason
    //
    let red_str = format!("#{}ff", red.to_hex_string());
    let green_str = format!("#{}ff", green.to_hex_string());

    let d_style = create::make_style("droneStyle", &red_str, 4.);
    let p_style = create::make_style("planeStyle", &green_str, 4.);

    // Create `Placemark` for each trajectory
    //
    let drone = create::from_traj_to_placemark(&drone_id, &drones, "droneStyle")?;
    let plane = create::from_traj_to_placemark(&prox_callsign, &planes, "planeStyle")?;

    // Not sure why there is no `Kml::Document()` like all others.
    //
    let doc = Document {
        attrs: [("name".into(), format!("{id}.kml"))].into(),
        elements: vec![d_style, p_style, drone, plane],
    };

    // Create the final KML
    //
    let kml = Kml::KmlDocument(KmlDocument {
        version: KmlVersion::V23,
        elements: vec![doc.into()],
        attrs: HashMap::new(),
    });

    Ok(kml.to_string())
}

/// Export all encounters
///
#[tracing::instrument(skip(ctx))]
async fn export_all_encounter(ctx: &Context, output: &PathBuf) -> Result<usize> {
    let client = ctx.db().await;

    assert!(output.is_dir(), "output must be a directory!");

    let list = data::fetch_all_en_id(&client).await?;
    let n = list.len();
    trace!("Found {n} encounters to export.");

    // Sanity check.
    //
    if n == 0 {
        return Err(format_err!("no encounters found!"));
    }

    // Run the big batch in chunk to limit CPU usage and number of threads.
    //
    for batch in &list.into_iter().chunks(ctx.pool_size) {
        // Generate KML data for each `en_id`
        //
        let kmls = batch
            .into_iter()
            .map(|en_id| async move {
                trace!("Generating KML for {en_id}");
                let ctx = ctx.clone();
                let id = en_id.clone();

                let kml =
                    tokio::spawn(async move { export_one_encounter(&ctx, &id).await.unwrap() })
                        .await
                        .unwrap();
                (en_id, kml)
            })
            .collect::<Vec<_>>();
        let kmls: Vec<(_, _)> = join_all(kmls).await;

        // Now write every file in the batch.
        //
        let hlist: Vec<_> = kmls
            .into_iter()
            .map(|(en_id, kml)| async move {
                let fname = output.join(en_id);
                fs::write(&fname, kml).await.unwrap();
            })
            .collect::<Vec<_>>();
        join_all(hlist).await;
    }

    eprintln!("Exporting all encounters in {output:?}... ");
    Ok(n)
}

// -----

/// Small internal module for clickhouse data fetching
///
mod data {
    use super::{DataPoint, Encounter};

    use chrono::{DateTime, Utc};
    use eyre::Result;
    use klickhouse::{Client, QueryBuilder, RawRow};
    use tracing::{debug, trace};

    #[tracing::instrument(skip(client))]
    pub(crate) async fn fetch_drones(
        client: &Client,
        journey: i32,
        drone_id: &str,
    ) -> Result<Vec<DataPoint>> {
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
        let drones = client.query_collect::<DataPoint>(q).await?;
        trace!("Found {} drone points for en_id {}", drones.len(), drone_id);

        debug!("drones={:?}", drones);

        Ok(drones)
    }

    #[tracing::instrument(skip(client))]
    pub(crate) async fn fetch_planes(
        client: &Client,
        prox_id: &str,
        first: DateTime<Utc>,
        last: DateTime<Utc>,
    ) -> Result<Vec<DataPoint>> {
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

        let q = QueryBuilder::new(rdp).arg(prox_id).arg(first).arg(last);
        let planes = client.query_collect::<DataPoint>(q).await?;
        trace!("Found {} plane points for id {}", planes.len(), prox_id);

        debug!("planes={:?}", planes);

        Ok(planes)
    }

    #[tracing::instrument(skip(client))]
    pub(crate) async fn fetch_one_encounter(client: &Client, id: &str) -> Result<Encounter> {
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

        Ok(res)
    }

    #[tracing::instrument(skip(client))]
    pub(crate) async fn fetch_all_en_id(client: &Client) -> Result<Vec<String>> {
        let r = r##"
SELECT
  en_id
FROM
  airprox_summary
ORDER BY
  en_id
    "##;
        let list = client
            .query_collect::<RawRow>(r)
            .await?
            .iter_mut()
            .map(|e| e.get(0))
            .collect::<Vec<String>>();
        Ok(list)
    }
}

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
    #[tracing::instrument]
    fn from_points_to_ls(points: &Vec<DataPoint>) -> eyre::Result<LineString> {
        let coords = points
            .into_iter()
            .map(|p| Coord::new(p.longitude, p.latitude, Some(p.altitude)))
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
    #[tracing::instrument]
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
