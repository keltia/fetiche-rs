//! Module for exporting encounters into KML files.
//!

mod create;
mod data;

use crate::cmds::Format;
use crate::config::Context;
use crate::error::Status;

use create::*;
use data::*;

use clap::Parser;
use colorsys::Rgb;
use eyre::Result;
use fetiche_common::DateOpts;
use futures::future::join_all;
use itertools::Itertools;
use kml::Kml::Document;
use kml::{Kml, KmlDocument, KmlVersion};
use regex::Regex;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, trace};

#[derive(Debug, Parser)]
pub struct ExpEncounterOpts {
    /// Export every encounter in its own file.
    #[clap(short = 'A', long)]
    all: bool,
    /// Export that Encounter ID
    #[clap(long)]
    id: Option<String>,
    /// Output Format
    #[clap(short = 'F', long, default_value = "kml", value_parser)]
    format: Format,
    /// Output file or directory.
    #[clap(short = 'o', long, default_value = ".")]
    output: Option<PathBuf>,
    /// Date (today, yesterday, etc.)
    #[clap(subcommand)]
    date: Option<DateOpts>,
}

/// Export one or all existing encounters as KML files into a single file/directory
///
#[tracing::instrument(skip(ctx))]
pub async fn export_encounters(ctx: &Context, opts: &ExpEncounterOpts) -> Result<()> {
    let client = ctx.db().await;

    // Check arguments
    //
    let all = opts.all;
    let id = &opts.id.clone();
    let output = &opts.output.clone();
    let date = opts.date.clone();

    // Can't have -A and an ID
    //
    if all && id.is_some() {
        return Err(Status::NoAllAndENID.into());
    }
    // Can't have a specific date and -A
    //
    if all && date.is_some() {
        return Err(Status::NoAllAndDate.into());
    }

    // Create the list of `en_id` to analyse.
    //
    let list = if all {
        trace!("Exporting all encounters.");

        fetch_all_en_id(&client).await?
    } else {
        trace!("Exporting some encounters.");

        match date {
            Some(date) => {
                trace!("Exporting all encounters for {date:?}...");
                fetch_encounters_on(&client, date).await?
            }
            None => {
                // A single en_id is requested
                //
                let en_id = match opts.id.clone() {
                    Some(id) => id,
                    None => return Err(Status::NoEncounterSpecified.into()),
                };
                vec![en_id]
            }
        }
    };

    // We need output to be specified and a directory
    //
    if let Some(output) = output {
        if !output.is_dir() {
            return Err(Status::NotADirectory(output.to_string_lossy().to_string()).into());
        }
        let _ = export_encounter_list(ctx, &list, output).await?;
    } else {
        return Err(Status::NoOutputDestination.into());
    }

    Ok(())
}

/// Export one single encounter
///
#[tracing::instrument(skip(ctx))]
async fn export_one_encounter(ctx: &Context, id: &str) -> Result<String> {
    let client = ctx.db().await;

    // Sanity check on the encounter ID.
    //
    let re = Regex::new(r##"^(?<name>[A-Z]{3})-(?<date>\d{8})-(?<journey>\d+)-(\d+)$"##)?;

    let (name, date, journey) = if let Some(caps) = re.captures(id) {
        let date = &caps["date"];

        let re = Regex::new(r##"^(?<year>\d{4})(?<month>\d{2})(?<day>\d{2})$"##)?;
        if re.captures(date).is_none() {
            return Err(Status::BadDateFormat(id.to_string()).into());
        }
        (
            &caps["name"].to_string(),
            date.to_string(),
            caps["journey"].parse::<i32>()?,
        )
    } else {
        return Err(Status::BadEncounterID(id.to_string()).into());
    };
    debug!("name: {}, date: {}, journey: {}", name, date, journey);

    let res = fetch_one_encounter(&client, id).await?;

    assert_eq!(res.en_id, id);
    assert_eq!(res.journey, journey);

    let encounter_timestamp = res.timestamp;
    let drone_id = res.drone_id.clone();

    // ICAO string is unique, whereas callsign can change
    //
    let prox_id = res.prox_id.clone();
    let prox_callsign = res.prox_callsign.clone();

    let drones = fetch_drones(&client, journey, &drone_id).await?;
    if drones.len() <= 1 {
        return Err(Status::NotEnoughData("drones".to_string()).into());
    }

    // Extract first and last timestamp to have a suitable interval for plane points.
    //
    let first = drones.first().unwrap().timestamp;
    let last = drones.last().unwrap().timestamp;

    // We use `prox_id` because this one does not change whereas callsign can and will
    //
    let planes = fetch_planes(&client, &prox_id, first, last).await?;
    if planes.len() <= 1 {
        return Err(Status::NotEnoughData("planes".to_string()).into());
    }

    // Pre-load default styles
    //
    let def_styles = default_styles();

    // Create `Placemark` for the encounter itself.
    //
    let point = from_point_to_placemark("Encounter", &res, "#msn_ylw-pushpin")?;

    // Create `Placemark` for each trajectory
    //
    let drone = from_traj_to_placemark(&drone_id, &drones, "#msn_ylw-pushpin0")?;
    let plane = from_traj_to_placemark(&prox_callsign, &planes, "#default")?;

    let mut elements = def_styles.clone();
    elements.push(drone);
    elements.push(plane);
    elements.push(point);

    let doc = Document {
        attrs: [
            ("name".into(), format!("{id}.kml")),
            ("time".into(), encounter_timestamp.to_string()),
        ]
            .into(),
        elements: elements.into(),
    };

    // Create the final KML
    //
    let kml = Kml::KmlDocument(KmlDocument {
        version: KmlVersion::V23,
        elements: vec![doc],
        ..Default::default()
    });

    Ok(kml.to_string())
}

/// Export a list of encounters.
///
#[tracing::instrument(skip(ctx))]
async fn export_encounter_list(
    ctx: &Context,
    list: &Vec<String>,
    output: &PathBuf,
) -> Result<usize> {
    assert!(output.is_dir(), "output must be a directory!");

    let n = list.len();
    trace!("Found {n} encounters to export.");

    // No new encounters to export is fine.
    //
    if n == 0 {
        return Ok(0);
    }

    // Run the big batch in chunk to limit CPU usage and number of threads.
    //
    for batch in &list.iter().chunks(ctx.pool_size) {
        // Generate KML data for each `en_id`
        //
        let kmls: Vec<_> = batch
            .into_iter()
            .map(|en_id| async move {
                trace!("Generating KML for {en_id}");
                let ctx = ctx.clone();
                let id = en_id.clone();
                let output = output.clone();

                tokio::spawn(async move {
                    let fname = output.join(&id);
                    match export_one_encounter(&ctx, &id).await {
                        Ok(res) => {
                            eprint!("{} ", fname.file_stem().unwrap().to_string_lossy());
                            let fname = fname.with_extension("kml");
                            let _ = fs::write(&fname, &res).await;
                        }
                        Err(e) => {
                            eprintln!("({e}");
                        }
                    };
                })
                    .await
                    .unwrap();
            })
            .collect();
        let _ = join_all(kmls).await;
    }

    eprintln!("Exporting {n} encounters in {output:?}... ");
    Ok(n)
}
