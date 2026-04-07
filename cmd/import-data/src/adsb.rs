use std::fmt::{Display, Formatter};
use std::fs::File;

use crate::{CmdError, Context, DBVars, make_query};

use cached::proc_macro::cached;
use eyre::Result;
use klickhouse::{QueryBuilder, Row};
use polars::frame::DataFrame;
use polars::prelude::{CsvReadOptions, NamedFrom, ParquetReader, SerReader, Series};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, trace};

/// One row of the `airplanes_raw` ClickHouse table.
///
/// Field names and types must match the table schema exactly.
/// All fields use default values (0, "", etc.) for missing data.
/// `site` is always set by us before insertion.
///
#[derive(Debug, Deserialize, Row, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AdsbRaw {
    #[klickhouse(rename = "Site")]
    site: i32,
    #[klickhouse(rename = "EmitterCategory")]
    emitter_category: i32,
    #[klickhouse(rename = "GBS")]
    gbs: i32,
    #[klickhouse(rename = "ModeA")]
    mode_a: String,
    #[klickhouse(rename = "TimeRecPosition")]
    time_rec_position: String,
    #[klickhouse(rename = "AircraftAddress")]
    aircraft_address: String,
    #[klickhouse(rename = "Latitude")]
    latitude: f64,
    #[klickhouse(rename = "Longitude")]
    longitude: f64,
    #[klickhouse(rename = "GeometricAltitude")]
    geometric_altitude: f64,
    #[klickhouse(rename = "FlightLevel")]
    flight_level: f64,
    #[klickhouse(rename = "BarometricVerticalRate")]
    barometric_vertical_rate: String,
    #[klickhouse(rename = "GeoVertRateExceeded")]
    geo_vert_rate_exceeded: String,
    #[klickhouse(rename = "GeometricVerticalRate")]
    geometric_vertical_rate: String,
    #[klickhouse(rename = "GroundSpeed")]
    ground_speed: f64,
    #[klickhouse(rename = "TrackAngle")]
    track_angle: f64,
    #[klickhouse(rename = "Callsign")]
    callsign: String,
    #[klickhouse(rename = "AircraftStopped")]
    aircraft_stopped: String,
    #[klickhouse(rename = "GroundTrackValid")]
    ground_track_valid: String,
    #[klickhouse(rename = "GroundHeadingProvided")]
    ground_heading_provided: String,
    #[klickhouse(rename = "MagneticNorth")]
    magnetic_north: String,
    #[klickhouse(rename = "SurfaceGroundSpeed")]
    surface_ground_speed: f64,
    #[klickhouse(rename = "SurfaceGroundTrack")]
    surface_ground_track: f64,
}

/// Import a single large CSV file into a given table in Clickhouse.
///
#[tracing::instrument(skip(ctx))]
pub async fn import_one_adsb(ctx: &Context, fname: &str, table: &str) -> Result<usize> {
    // Check extension, both csv & parquet are valid
    //
    let basename = check_basename(fname)?;
    let ext = check_extension(fname)?;

    // Retrieve site ID
    //
    let site = fetch_site_id(ctx, &basename).await?;

    debug!("site_name={} site_id={}", site.name, site.id);

    // Read the file into a DataFrame.
    //
    let df = match ext.as_str() {
        "csv" => read_one_csv(fname)?,
        "parquet" => read_one_parquet(fname).await?,
        _ => unreachable!(),
    };

    let total_rows = df.height();
    debug!(
        "fname={fname} rows={total_rows} threshold={}",
        ctx.batch_size
    );

    let mut batch_num = 0usize;
    let mut offset = 0usize;

    //    let mut result = Vec::with_capacity(total_rows);
    // Proceed by batch
    //
    while offset < total_rows {
        let batch_size = ctx.batch_size.min(total_rows - offset);
        let mut batch = df.slice(offset as i64, batch_size);

        // Prepend the site_id column so it is the first column
        //
        let site_col = Series::new("Site".into(), vec![Some(site.id); batch_size]);
        batch.insert_column(0, site_col.into())?;

        debug!("batch={batch_num} rows={batch_size} offset={offset}");

        let rows = insert_batch(ctx, &table, &batch).await?;
        assert_eq!(rows, batch_size);

        offset += batch_size;
        batch_num += 1;
    }

    debug!("import batches={batch_num} rows={total_rows}");
    Ok(total_rows)
}

/// Read the full CSV into a DataFrame.
///
#[tracing::instrument]
pub fn read_one_csv(fname: &str) -> Result<DataFrame> {
    let path = PathBuf::from(&fname);
    let df = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(path))?
        .finish()?;
    Ok(df)
}

/// Read the full Parquet into a DataFrame.
///
#[tracing::instrument]
pub async fn read_one_parquet(fname: &str) -> Result<DataFrame> {
    let fh = File::open(fname)?;
    let df = ParquetReader::new(fh).finish()?;
    Ok(df)
}

/// Check basename for a specific, site-based pattern
///
#[tracing::instrument]
fn check_basename(fname: &str) -> Result<String> {
    // Check basename
    //
    let sname = Path::new(fname).file_stem().unwrap().to_str().unwrap();
    debug!("sname={}", sname);

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
    Ok(basename)
}

/// Check extension, we accept CSV and Parquet
///
#[tracing::instrument]
fn check_extension(fname: &str) -> Result<String> {
    let ext = if let Some(ext) = Path::new(fname).extension().unwrap().to_str() {
        ext
    } else {
        return Err(CmdError::BadFilenamePattern("No file extension".into()).into());
    };
    if ext != "csv" && ext != "parquet" {
        return Err(CmdError::NeedsCsvOrParquet(ext.into()).into());
    }
    Ok(ext.to_string())
}

/// Convert one DataFrame batch into `AdsbRaw` rows and bulk-insert into ClickHouse.
///
#[tracing::instrument(skip(df))]
async fn insert_batch(ctx: &Context, table: &str, df: &DataFrame) -> Result<usize> {
    let db = ctx.db().await;

    let n = df.height();

    // Required column — added by us, always present
    //
    let s_site = df.column("Site").ok();
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

    // Extract typed views upfront — one HashMap lookup per column, not per row.
    // Types match what polars infers from the CSV (all bare integers → Int64,
    // all decimals → Float64; Boolean/u8 are NOT inferred from 0/1 integers).
    //
    let c_site = s_site.and_then(|s| s.i32().ok());
    let c_emitter = s_emitter.and_then(|s| s.i32().ok());
    let c_gbs = s_gbs.and_then(|s| s.i32().ok());
    let c_mode_a = s_mode_a.and_then(|s| s.i64().ok()); // Int64 → to_string
    let c_time_rec = s_time_rec.and_then(|s| s.str().ok()); // Datetime('μs')
    let c_addr = s_addr.and_then(|s| s.str().ok());
    let c_lat = s_lat.and_then(|s| s.f64().ok());
    let c_lon = s_lon.and_then(|s| s.f64().ok());
    let c_geo_alt = s_geo_alt.and_then(|s| s.f64().ok());
    let c_fl = s_fl.and_then(|s| s.f64().ok());
    let c_baro_vr = s_baro_vr.and_then(|s| s.f64().ok()); // Float64 → to_string
    let c_geo_vre = s_geo_vre.and_then(|s| s.str().ok());
    let c_geo_vr = s_geo_vr.and_then(|s| s.str().ok());
    let c_gs = s_gs.and_then(|s| s.f64().ok());
    let c_ta = s_ta.and_then(|s| s.f64().ok());
    let c_cs = s_cs.and_then(|s| s.str().ok());
    let c_stopped = s_stopped.and_then(|s| s.i64().ok()); // Int64 → to_string
    let c_gtv = s_gtv.and_then(|s| s.i64().ok()); // Int64 → to_string
    let c_ghp = s_ghp.and_then(|s| s.i64().ok()); // Int64 → to_string
    let c_mn = s_mn.and_then(|s| s.i64().ok()); // Int64 → to_string
    let c_sgs = s_sgs.and_then(|s| s.f64().ok()); // Float64 → cast to f32
    let c_sgt = s_sgt.and_then(|s| s.f64().ok()); // Float64 → cast to f32

    let rows: Vec<AdsbRaw> = (0..n)
        .map(|i| AdsbRaw {
            site: c_site.and_then(|c| c.get(i)).map(|v| v as i32).unwrap_or(0),
            emitter_category: c_emitter
                .and_then(|c| c.get(i))
                .map(|v| v as i32)
                .unwrap_or(0),
            gbs: c_gbs.and_then(|c| c.get(i)).map(|v| v as i32).unwrap_or(0),
            mode_a: c_mode_a
                .and_then(|c| c.get(i))
                .map(|v| v.to_string())
                .unwrap_or_default(),
            time_rec_position: c_time_rec
                .and_then(|c| c.get(i))
                .map(String::from)
                .unwrap_or_default(),
            aircraft_address: c_addr
                .and_then(|c| c.get(i))
                .map(String::from)
                .unwrap_or_default(),
            latitude: c_lat.and_then(|c| c.get(i)).unwrap_or(0.0),
            longitude: c_lon.and_then(|c| c.get(i)).unwrap_or(0.0),
            geometric_altitude: c_geo_alt.and_then(|c| c.get(i)).unwrap_or(0.0),
            flight_level: c_fl.and_then(|c| c.get(i)).unwrap_or(0.0),
            barometric_vertical_rate: c_baro_vr
                .and_then(|c| c.get(i))
                .map(|v| v.to_string())
                .unwrap_or_default(),
            geo_vert_rate_exceeded: c_geo_vre
                .and_then(|c| c.get(i))
                .map(String::from)
                .unwrap_or_default(),
            geometric_vertical_rate: c_geo_vr
                .and_then(|c| c.get(i))
                .map(String::from)
                .unwrap_or_default(),
            ground_speed: c_gs.and_then(|c| c.get(i)).unwrap_or(0.0),
            track_angle: c_ta.and_then(|c| c.get(i)).unwrap_or(0.0),
            callsign: c_cs
                .and_then(|c| c.get(i))
                .map(String::from)
                .unwrap_or_default(),
            aircraft_stopped: c_stopped
                .and_then(|c| c.get(i))
                .map(|v| v.to_string())
                .unwrap_or_default(),
            ground_track_valid: c_gtv
                .and_then(|c| c.get(i))
                .map(|v| v.to_string())
                .unwrap_or_default(),
            ground_heading_provided: c_ghp
                .and_then(|c| c.get(i))
                .map(|v| v.to_string())
                .unwrap_or_default(),
            magnetic_north: c_mn
                .and_then(|c| c.get(i))
                .map(|v| v.to_string())
                .unwrap_or_default(),
            surface_ground_speed: c_sgs.and_then(|c| c.get(i)).unwrap_or(0.0),
            surface_ground_track: c_sgt.and_then(|c| c.get(i)).unwrap_or(0.0),
        })
        .collect();

    debug!("{:?}", &rows);

    if !ctx.dry_run {
        let _ = db
            .insert_native_block(format!("INSERT INTO {table} FORMAT native"), rows)
            .await?;
        trace!("inserted: rows={n}table={table}");
    } else {
        eprintln!("I almost imported the data.");
        trace!("dry-run: rows={n} table={table}");
    }

    Ok(n)
}

/// Query result.
///
#[derive(Clone, Debug, Deserialize, Row)]
struct Site {
    /// Site ID
    id: i32,
    /// Site name
    name: String,
}

impl Display for Site {
    /// Implement `Display`.  Just return the name for now.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Fetch the site ID from the databases with the specified basename
///
#[tracing::instrument(skip(ctx))]
#[cached(key = "String", result = true, convert = r#"{format!("{}", name)}"#)]
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
