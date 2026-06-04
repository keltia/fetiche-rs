//! Benchmark between csv to struct and through DataFrame with polars
//!
//!```text
//! Timer precision: 100 ns
//! ser               fastest       │ slowest       │ median        │ mean          │ samples │ iters
//! ├─ csv_to_df      60.69 µs      │ 1.563 ms      │ 75.49 µs      │ 95.01 µs      │ 100     │ 100
//! ╰─ csv_to_struct  18.89 µs      │ 59.69 µs      │ 19.19 µs      │ 20.13 µs      │ 100     │ 100
//! ```
//!
use chrono::{DateTime, NaiveDateTime, Utc};
use divan::Bencher;
use serde::{Deserialize, Serialize};
use std::hint::black_box;
use std::io::Cursor;

fn main() {
    divan::main();
}

const CSV_DATA: &str = r##"
EmitterCategory,GBS,ModeA,TimeRecPosition,AircraftAddress,Latitude,Longitude,GeometricAltitude,FlightLevel,BarometricVerticalRate,GeoVertRateExceeded,GeometricVerticalRate,GroundSpeed,TrackAngle,Callsign,AircraftStopped,GroundTrackValid,GroundHeadingProvided,MagneticNorth,SurfaceGroundSpeed,SurfaceGroundTrack
5,1,2036,2026-03-23 23:59:59.730,4243FA,63.9887007326,-22.6054000854,,,,,,,,'AZG5721 ',0,1,0,0,22.000,0.000
3,0,3210,2026-03-23 23:59:59.710,4CC570,63.8257714547,-22.6040403731,2400.00,32.00,-318.75,,,157.76,354.62,'ICE213  ',,,,,,
5,0,3247,2026-03-23 23:59:59.660,407EA0,63.9427107386,-22.0179198682,11050.00,122.50,2687.50,,,390.45,124.09,'DHK3999 ',,,,,,
3,0,3254,2026-03-23 23:59:59.630,4D2434,63.4267722443,-20.2237426490,24775.00,264.50,-2368.75,,,380.79,291.13,'WZZ21YH ',,,,,,
5,1,2036,2026-03-24 00:00:00.840,4243FA,63.9888038300,-22.6054000854,,,,,,,,'AZG5721 ',0,1,0,0,22.000,0.000
3,0,3210,2026-03-24 00:00:00.810,4CC570,63.8265628740,-22.6042601466,2400.00,32.00,-318.75,,,157.54,355.34,'ICE213  ',,,,,,
5,0,3247,2026-03-24 00:00:00.810,407EA0,63.9414824545,-22.0138549805,11075.00,123.00,2818.75,,,391.11,124.21,'DHK3999 ',,,,,,
3,0,3254,2026-03-24 00:00:00.740,4D2434,63.4274595603,-20.2279428206,24725.00,264.00,-2368.75,,,380.79,291.13,'WZZ21YH ',,,,,,
3,1,2264,2026-03-24 00:00:01.980,4D24D4,63.9945487864,-22.6178125106,,,,,,,,'WZZ6EU  ',1,1,0,0,0.000,95.625
5,1,2036,2026-03-24 00:00:01.930,4243FA,63.9889253676,-22.6054136641,,,,,,,,'AZG5721 ',0,1,0,0,22.000,0.000"##;

mod parse_str_date {
    use chrono::{DateTime, NaiveDateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S%.f";

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}

#[divan::bench]
fn csv_to_struct(bencher: Bencher) {
    use csv::ReaderBuilder;

    fn deser_csv() -> eyre::Result<()> {
        #[derive(Debug, Default, Deserialize, Serialize)]
        struct AdsbRaw {
            #[serde(rename = "EmitterCategory")]
            emitter_category: i32,
            #[serde(rename = "GBS")]
            gbs: i32,
            #[serde(rename = "ModeA")]
            mode_a: String,
            #[serde(rename = "TimeRecPosition", with = "parse_str_date")]
            time_rec_position: DateTime<Utc>,
            #[serde(rename = "AircraftAddress")]
            aircraft_address: String,
            #[serde(rename = "Latitude")]
            latitude: f64,
            #[serde(rename = "Longitude")]
            longitude: f64,
            #[serde(rename = "GeometricAltitude", default = "Default::default")]
            geometric_altitude: Option<f64>,
            #[serde(rename = "FlightLevel")]
            flight_level: Option<f64>,
            #[serde(rename = "BarometricVerticalRate")]
            barometric_vertical_rate: String,
            #[serde(rename = "GeoVertRateExceeded")]
            geo_vert_rate_exceeded: String,
            #[serde(rename = "GeometricVerticalRate")]
            geometric_vertical_rate: String,
            #[serde(rename = "GroundSpeed")]
            ground_speed: Option<f64>,
            #[serde(rename = "TrackAngle")]
            track_angle: Option<f64>,
            #[serde(rename = "Callsign")]
            callsign: String,
            #[serde(rename = "AircraftStopped")]
            aircraft_stopped: String,
            #[serde(rename = "GroundTrackValid")]
            ground_track_valid: String,
            #[serde(rename = "GroundHeadingProvided")]
            ground_heading_provided: String,
            #[serde(rename = "MagneticNorth")]
            magnetic_north: String,
            #[serde(rename = "SurfaceGroundSpeed")]
            surface_ground_speed: Option<f64>,
            #[serde(rename = "SurfaceGroundTrack")]
            surface_ground_track: Option<f64>,
        }

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(CSV_DATA.as_bytes());
        let _rows = rdr
            .deserialize::<AdsbRaw>()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(())
    }

    bencher.bench_local(move || black_box(deser_csv().unwrap()));
}

#[divan::bench]
fn csv_to_df(bencher: Bencher) {
    use polars::prelude::*;

    fn deser_csv() -> eyre::Result<()> {
        #[derive(Debug, Default, Deserialize, Serialize)]
        struct AdsbRaw {
            #[serde(rename = "EmitterCategory")]
            emitter_category: i32,
            #[serde(rename = "GBS")]
            gbs: i32,
            #[serde(rename = "ModeA")]
            mode_a: String,
            #[serde(rename = "TimeRecPosition")]
            time_rec_position: DateTime<Utc>,
            #[serde(rename = "AircraftAddress")]
            aircraft_address: String,
            #[serde(rename = "Latitude")]
            latitude: f64,
            #[serde(rename = "Longitude")]
            longitude: f64,
            #[serde(rename = "GeometricAltitude", default = "Default::default")]
            geometric_altitude: f64,
            #[serde(rename = "FlightLevel")]
            flight_level: f64,
            #[serde(rename = "BarometricVerticalRate")]
            barometric_vertical_rate: String,
            #[serde(rename = "GeoVertRateExceeded")]
            geo_vert_rate_exceeded: String,
            #[serde(rename = "GeometricVerticalRate")]
            geometric_vertical_rate: String,
            #[serde(rename = "GroundSpeed")]
            ground_speed: f64,
            #[serde(rename = "TrackAngle")]
            track_angle: f64,
            #[serde(rename = "Callsign")]
            callsign: String,
            #[serde(rename = "AircraftStopped")]
            aircraft_stopped: String,
            #[serde(rename = "GroundTrackValid")]
            ground_track_valid: String,
            #[serde(rename = "GroundHeadingProvided")]
            ground_heading_provided: String,
            #[serde(rename = "MagneticNorth")]
            magnetic_north: String,
            #[serde(rename = "SurfaceGroundSpeed")]
            surface_ground_speed: f64,
            #[serde(rename = "SurfaceGroundTrack")]
            surface_ground_track: f64,
        }

        let rdr = Cursor::new(CSV_DATA);
        let df = CsvReadOptions::default()
            .into_reader_with_file_handle(rdr)
            .finish()?;

        let n = df.height();
        // Required column — added by us, always present
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

        // Extract typed views upfront — one HashMap lookup per column, not per row.
        // Types match what polars infers from the CSV (all bare integers → Int64,
        // all decimals → Float64; Boolean/u8 are NOT inferred from 0/1 integers).
        //
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

        let _rows: Vec<AdsbRaw> = (0..n)
            .map(|i| AdsbRaw {
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
                    .map(|s| {
                        let d = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
                            .unwrap_or_default();
                        d.and_utc()
                    })
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
        Ok(())
    }

    bencher.bench_local(move || black_box(deser_csv().unwrap()));
}
