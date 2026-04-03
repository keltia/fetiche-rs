//! This is the Rust equivalent of [import-adsb.py] with batching capabilities
//!

use crate::cmds::{CmdError, DBVars};
use crate::make_query;
use crate::runtime::Context;
use chrono::{DateTime, Utc};
use clap::Parser;
use eyre::Result;
use klickhouse::{QueryBuilder, Row};
use polars::prelude::{LazyCsvReader, LazyFileListReader, NamedFrom, Series};
use regex::Regex;
use serde::Deserialize;
use std::path::Path;
use tracing::{debug, trace};

/// `import adsb` options
///
#[derive(Debug, Parser)]
pub struct AdsbOpts {
    /// Table name
    #[clap(short = 'T', long)]
    pub table: String,
    /// Batch import by this number of lines
    #[clap(short = 't', long, default_value = "500000")]
    pub threshold: usize,
    /// Filename
    pub fname: String,
}

#[derive(Debug, Deserialize, Row)]
struct Site {
    /// Site ID
    id: i32,
    /// Site name
    name: String,
}

/// One row of the `airplanes_raw` ClickHouse table.
///
/// Field names and types must match the table schema exactly.
/// All CSV-sourced columns are `Option<T>` to handle sparse data;
/// `site` is always set by us before insertion.
///
#[derive(Debug, Row)]
struct AdsbRaw {
    site: i32,
    #[klickhouse(rename = "EmitterCategory")]
    emitter_category: Option<u8>,
    #[klickhouse(rename = "GBS")]
    gbs: Option<u8>,
    #[klickhouse(rename = "ModeA")]
    mode_a: Option<String>,
    #[klickhouse(rename = "TimeRecPosition")]
    time_rec_position: Option<DateTime<Utc>>,
    #[klickhouse(rename = "AircraftAddress")]
    aircraft_address: Option<String>,
    #[klickhouse(rename = "Latitude")]
    latitude: Option<f64>,
    #[klickhouse(rename = "Longitude")]
    longitude: Option<f64>,
    #[klickhouse(rename = "GeometricAltitude")]
    geometric_altitude: Option<f64>,
    #[klickhouse(rename = "FlightLevel")]
    flight_level: Option<f64>,
    #[klickhouse(rename = "BarometricVerticalRate")]
    barometric_vertical_rate: Option<String>,
    #[klickhouse(rename = "GeoVertRateExceeded")]
    geo_vert_rate_exceeded: Option<String>,
    #[klickhouse(rename = "GeometricVerticalRate")]
    geometric_vertical_rate: Option<String>,
    #[klickhouse(rename = "GroundSpeed")]
    ground_speed: Option<f64>,
    #[klickhouse(rename = "TrackAngle")]
    track_angle: Option<f64>,
    #[klickhouse(rename = "Callsign")]
    callsign: Option<String>,
    #[klickhouse(rename = "AircraftStopped")]
    aircraft_stopped: Option<String>,
    #[klickhouse(rename = "GroundTrackValid")]
    ground_track_valid: Option<String>,
    #[klickhouse(rename = "GroundHeadingProvided")]
    ground_heading_provided: Option<String>,
    #[klickhouse(rename = "MagneticNorth")]
    magnetic_north: Option<String>,
    #[klickhouse(rename = "SurfaceGroundSpeed")]
    surface_ground_speed: Option<f32>,
    #[klickhouse(rename = "SurfaceGroundTrack")]
    surface_ground_track: Option<f32>,
}

/// Import a single large CSV file into a given table in Clickhouse.
///
#[tracing::instrument(skip(ctx))]
pub async fn import_adsb(ctx: &Context, opts: &AdsbOpts) -> Result<()> {
    let table = opts.table.clone();
    let fname = opts.fname.clone();

    let sname = Path::new(fname.as_str())
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    debug!("sname={}", sname);

    // Check input file
    //
    let ext = if let Some(ext) = Path::new(fname.as_str()).extension().unwrap().to_str() {
        ext
    } else {
        return Err(CmdError::NeedCsvFile("No file extension".into()).into());
    };
    if ext != "csv" {
        return Err(CmdError::NeedCsvFile(ext.into()).into());
    }

    // Filename should be formatted like this: `<basename>_YYYY-MM-DD`
    //
    let re = Regex::new(
        r##"^(?<basename>[[:alnum:]]+)_(?<year>[[:digit:]]{4})-(?<month>[[:digit:]]{2})-(?<day>[[:digit:]]{2})$"##,
    )?;
    let (basename, year, month, day) = if let Some(caps) = re.captures(sname) {
        (
            caps[1].to_string(),
            caps[2].parse::<u32>()?,
            caps[3].parse::<u32>()?,
            caps[4].parse::<u32>()?,
        )
    } else {
        return Err(CmdError::BadFilenamePattern(sname.into()).into());
    };
    trace!("handling basename={basename} from={year}-{month}-{day}");

    // Retrieve site ID
    //
    let site = fetch_site_id(ctx, &basename).await?;
    debug!("site_name={} site_id={}", site.name, site.id);

    // Read the full CSV into a DataFrame.
    //
    let df = LazyCsvReader::new(fname.as_str().into())
        .map_parse_options(|opts| opts.with_try_parse_dates(true))
        .finish()?
        .collect()?;

    let total_rows = df.height();
    let threshold = opts.threshold;
    debug!("loaded {total_rows} rows from {fname}");

    let mut batch_num = 0usize;
    let mut offset = 0usize;

    // Proceed by batch
    //
    while offset < total_rows {
        let batch_size = threshold.min(total_rows - offset);
        let mut batch = df.slice(offset as i64, batch_size);

        // Prepend the site_id column so it is the first column
        //
        let site_col = Series::new("site".into(), vec![site.id; batch_size]);
        batch.insert_column(0, site_col.into())?;

        debug!("batch={batch_num} rows={batch_size} offset={offset},");

        insert_batch(ctx, &table, &batch).await?;

        offset += batch_size;
        batch_num += 1;
    }

    debug!("import batches={batch_num} rows={total_rows}");
    Ok(())
}

/// Convert one DataFrame batch into `AdsbRaw` rows and bulk-insert into ClickHouse.
///
#[tracing::instrument(skip(ctx, df))]
async fn insert_batch(ctx: &Context, table: &str, df: &polars::prelude::DataFrame) -> Result<()> {
    let db = ctx.db().await;
    let n = df.height();

    // Required column — added by us, always present
    //
    let site = df.column("site")?.i32()?;

    // Optional columns — use `.ok()` so a missing column in the CSV just
    // produces `None` values rather than an error
    //
    let s_emitter = df.column("EmitterCategory").ok();
    let s_gbs = df.column("GBS").ok();
    let s_mode_a = df.column("ModeA").ok();
    let s_time_rec = df.column("TimeRecPosition").ok();
    let s_addr = df.column("AircraftAddress").ok();
    let s_lat = df.column("Latitude").ok();
    let s_lon = df.column("Longitude").ok();
    let s_geo_alt = df.column("GeometricAltitude").ok();
    let s_fl = df.column("FlightLevel").ok();
    let s_baro_vr = df.column("BarometricVerticalRate").ok();
    let s_geo_vre = df.column("GeoVertRateExceeded").ok();
    let s_geo_vr = df.column("GeometricVerticalRate").ok();
    let s_gs = df.column("GroundSpeed").ok();
    let s_ta = df.column("TrackAngle").ok();
    let s_cs = df.column("Callsign").ok();
    let s_stopped = df.column("AircraftStopped").ok();
    let s_gtv = df.column("GroundTrackValid").ok();
    let s_ghp = df.column("GroundHeadingProvided").ok();
    let s_mn = df.column("MagneticNorth").ok();
    let s_sgs = df.column("SurfaceGroundSpeed").ok();
    let s_sgt = df.column("SurfaceGroundTrack").ok();

    // Extract typed views upfront — one HashMap lookup per column, not per row
    //
    let c_emitter = s_emitter.and_then(|s| s.u8().ok());
    let c_gbs = s_gbs.and_then(|s| s.u8().ok());
    let c_mode_a = s_mode_a.and_then(|s| s.str().ok());
    let c_time_rec = s_time_rec.and_then(|s| s.datetime().ok()); // &DatetimeChunked
    let c_addr = s_addr.and_then(|s| s.str().ok());
    let c_lat = s_lat.and_then(|s| s.f64().ok());
    let c_lon = s_lon.and_then(|s| s.f64().ok());
    let c_geo_alt = s_geo_alt.and_then(|s| s.f64().ok()); // Option<f64>
    let c_fl = s_fl.and_then(|s| s.f64().ok()); // Option<f64>
    let c_baro_vr = s_baro_vr.and_then(|s| s.str().ok()); // Option<String>
    let c_geo_vre = s_geo_vre.and_then(|s| s.str().ok());
    let c_geo_vr = s_geo_vr.and_then(|s| s.str().ok()); // Option<String>
    let c_gs = s_gs.and_then(|s| s.f64().ok()); // Option<f64>
    let c_ta = s_ta.and_then(|s| s.f64().ok()); // Option<f64>
    let c_cs = s_cs.and_then(|s| s.str().ok());
    let c_stopped = s_stopped.and_then(|s| s.str().ok());
    let c_gtv = s_gtv.and_then(|s| s.str().ok());
    let c_ghp = s_ghp.and_then(|s| s.str().ok());
    let c_mn = s_mn.and_then(|s| s.str().ok());
    let c_sgs = s_sgs.and_then(|s| s.f32().ok());
    let c_sgt = s_sgt.and_then(|s| s.f32().ok());

    let rows: Vec<AdsbRaw> = (0..n)
        .map(|i| AdsbRaw {
            site: site.get(i).unwrap_or_default(),
            emitter_category: c_emitter.and_then(|c| c.get(i)),
            gbs: c_gbs.and_then(|c| c.get(i)),
            mode_a: c_mode_a.and_then(|c| c.get(i)).map(String::from),
            time_rec_position: c_time_rec.and_then(|c| {
                // DatetimeChunked = Logical<DatetimeType, Int64Type>; .phys is Int64Chunked
                c.phys.get(i).and_then(|ts| {
                    use polars::prelude::TimeUnit;
                    let us = match c.time_unit() {
                        TimeUnit::Milliseconds => ts * 1_000,
                        TimeUnit::Microseconds => ts,
                        TimeUnit::Nanoseconds => ts / 1_000,
                    };
                    DateTime::from_timestamp_micros(us)
                })
            }),
            aircraft_address: c_addr.and_then(|c| c.get(i)).map(String::from),
            latitude: c_lat.and_then(|c| c.get(i)),
            longitude: c_lon.and_then(|c| c.get(i)),
            geometric_altitude: c_geo_alt.and_then(|c| c.get(i)),
            flight_level: c_fl.and_then(|c| c.get(i)),
            barometric_vertical_rate: c_baro_vr.and_then(|c| c.get(i)).map(String::from),
            geo_vert_rate_exceeded: c_geo_vre.and_then(|c| c.get(i)).map(String::from),
            geometric_vertical_rate: c_geo_vr.and_then(|c| c.get(i)).map(String::from),
            ground_speed: c_gs.and_then(|c| c.get(i)),
            track_angle: c_ta.and_then(|c| c.get(i)),
            callsign: c_cs.and_then(|c| c.get(i)).map(String::from),
            aircraft_stopped: c_stopped.and_then(|c| c.get(i)).map(String::from),
            ground_track_valid: c_gtv.and_then(|c| c.get(i)).map(String::from),
            ground_heading_provided: c_ghp.and_then(|c| c.get(i)).map(String::from),
            magnetic_north: c_mn.and_then(|c| c.get(i)).map(String::from),
            surface_ground_speed: c_sgs.and_then(|c| c.get(i)),
            surface_ground_track: c_sgt.and_then(|c| c.get(i)),
        })
        .collect();

    db.insert_native_block(format!("INSERT INTO {table} FORMAT native"), rows)
        .await?;

    trace!("inserted {} rows into {table}", n);
    Ok(())
}

/// Fetch the site ID from the databases with the specified basename
///
#[tracing::instrument(skip(ctx))]
async fn fetch_site_id(ctx: &Context, name: &str) -> Result<Site> {
    let db = ctx.db().await;
    let dbvars = DBVars::from_ctx(&ctx);

    let r = make_query!(
        r##"
SELECT id, name
FROM {workdb}.sites
WHERE basename = $1
    "##,
        dbvars
    );
    let q = QueryBuilder::new(&r).arg(name);
    let site = db.query_one::<Site>(q).await?;
    debug!("basename={name} id={} name={}", site.id, site.name);

    Ok(site)
}
