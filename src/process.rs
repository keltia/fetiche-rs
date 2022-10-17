//! Module to load and process the Aeroscope data coming from ASD and generate
//! CSV data Cat21-like
//!

use std::io::Read;

use anyhow::Result;
use chrono::{DateTime, Utc};
use csv::{Reader, WriterBuilder};
use log::trace;
use serde::{Deserialize, Serialize};

/// Our input structure from the csv file coming out of the aeroscope
///
#[derive(Debug, Deserialize)]
pub struct In {
    // $1
    pub aeroscope_id: String,
    // $2
    #[serde(rename = "aeroscope_position.latitude")]
    pub aeroscope_latitude: f32,
    // $3
    #[serde(rename = "aeroscope_position.longitude")]
    pub aeroscope_longitude: f32,
    // $4
    pub altitude: f32,
    // $5
    pub azimuth: f32,
    // $6
    #[serde(rename = "coordinate.latitude")]
    pub coordinate_latitude: f32,
    // $7
    #[serde(rename = "coordinate.longitude")]
    pub coordinate_longitude: f32,
    // $8
    pub distance: f32,
    // $9
    pub drone_id: String,
    // $10
    pub drone_type: String,
    // $11
    pub flight_id: String,
    // $12
    #[serde(rename = "home_location.latitude")]
    pub home_latitude: f32,
    // $13
    #[serde(rename = "home_location.longitude")]
    pub home_longitude: f32,
    // $14
    #[serde(rename = "pilot_position.latitude")]
    pub pilot_latitude: f32,
    // $15
    #[serde(rename = "pilot_position.longitude")]
    pub pilot_longitude: f32,
    // $16
    pub receive_date: String,
    // $17
    pub speed: f32,
}

/// Load and transform data from a Reader
///
pub fn process_data<T>(rdr: &mut Reader<T>) -> Result<Vec<Cat21>>
where
    T: Read,
{
    trace!("Reading & transforming…");
    let mut cnt = 1;
    let res: Vec<Cat21> = rdr
        .deserialize()
        .into_iter()
        .map(|rec| {
            let line: In = rec.unwrap();
            let mut line: Cat21 = Cat21::from(line);
            line.rec_num = cnt;
            cnt += 1;
            line
        })
        .collect();
    Ok(res)
}

/// Our pseudo cat21 csv output, we add the mapping from the awk script in comment
///
/// SAC:SIC:ALT_GEO_FT:POS_LAT_DEG:POS_LONG_DEG:ALT_BARO_FT:TOD:REC_TIME_POSIX:REC_TIME_MS:
/// EMITTER_CATEGORY:DIFFERENTIAL_CORRECTION:GROUND_BIT:SIMULATED_TARGET:TEST_TARGET:FROM_FFT:
/// SELECTED_ALT_CAPABILITY:SPI:LINK_TECHNOLOGY_CDTI:LINK_TECHNOLOGY_MDS:LINK_TECHNOLOGY_UAT:
/// LINK_TECHNOLOGY_VDL:LINK_TECHNOLOGY_OTHER:DESCRIPTOR_ATP:ALT_REPORTING_CAPABILITY_FT:
/// TARGET_ADDR:CAT:LINE_ID:DS_ID:REPORT_TYPE:TOD_CALCULATED:CALLSIGN:GROUNDSPEED_KT:T
/// RACK_ANGLE_DEG:REC_NUM
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Cat21 {
    // $a
    pub sac: usize,
    // $b
    pub sic: usize,
    // $c
    pub alt_geo_ft: u32,
    // $c1 (these should be a Position struct)
    pub pos_lat_deg: f32,
    // $c2
    pub pos_long_deg: f32,
    // $c3
    pub alt_baro_ft: u32,
    // $d
    pub tod: u32,
    // $d1
    pub rec_time_posix: u32,
    // $d2
    pub rec_time_ms: u32,
    // $e
    pub emitter_category: usize,
    // $f
    pub differential_correction: String,
    // $g
    pub ground_bit: String,
    // $h
    pub simulated_target: String,
    // $i
    pub test_target: String,
    // $j
    pub from_ft: String,
    // $k
    pub selected_alt_capability: String,
    // $l
    pub spi: String,
    // $l1 (these ought to be an enum)
    pub link_technology_cddi: String,
    // $l2
    pub link_technology_mds: String,
    // $l3
    pub link_technology_uat: String,
    // $l4
    pub link_technology_vdl: String,
    // $l5
    pub link_technology_other: String,
    // $m
    pub descriptor_atp: usize,
    // $n
    pub alt_reporting_capability_ft: usize,
    // $o
    pub target_addr: u32,
    // $p
    pub cat: usize,
    // $q
    pub line_id: usize,
    // $r
    pub ds_id: usize,
    // $s
    pub report_type: usize,
    // $t
    pub tod_calculated: String,
    // $u
    pub callsign: String,
    // $v
    pub groundspeed_kt: f32,
    // $w
    pub track_angle_deg: f32,
    // $y
    pub rec_num: usize,
}

impl From<In> for Cat21 {
    /// Makes the loading and transformations
    ///
    /// The default values are arbitrary and taken from the original `aeroscope.sh` scipt
    /// by Marc Gravis.
    ///
    fn from(line: In) -> Self {
        let tod = line.receive_date.parse::<DateTime<Utc>>().unwrap();
        let tod = tod.format("%s").to_string().parse::<u32>().unwrap();
        Cat21 {
            sac: 8,
            sic: 200,
            alt_geo_ft: (3.28084 * line.altitude) as u32,
            pos_lat_deg: line.coordinate_latitude,
            pos_long_deg: line.coordinate_longitude,
            alt_baro_ft: (3.28084 * line.altitude) as u32,
            tod: 128 * (tod % 86400),
            rec_time_posix: tod,
            rec_time_ms: 0,
            emitter_category: 13,
            differential_correction: "N".to_string(),
            ground_bit: "N".to_string(),
            simulated_target: "N".to_string(),
            test_target: "N".to_string(),
            from_ft: "N".to_string(),
            selected_alt_capability: "N".to_string(),
            spi: "N".to_string(),
            link_technology_cddi: "N".to_string(),
            link_technology_mds: "N".to_string(),
            link_technology_uat: "N".to_string(),
            link_technology_vdl: "N".to_string(),
            link_technology_other: "N".to_string(),
            descriptor_atp: 1,
            alt_reporting_capability_ft: 0,
            target_addr: 623615,
            cat: 21,
            line_id: 1,
            ds_id: 18,
            report_type: 3,
            tod_calculated: "N".to_string(),
            // We do truncate the drone_id for privacy reasons
            callsign: line.drone_id[2..10].to_owned(),
            groundspeed_kt: 0.54 * line.speed,
            track_angle_deg: line.azimuth,
            rec_num: 1,
        }
    }
}

/// Output the final csv file with a different delimiter 'now ":")
///
pub fn prepare_csv(data: Vec<Cat21>) -> Result<String> {
    trace!("Generating output…");
    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .delimiter(b':')
        .has_headers(true)
        .from_writer(vec![]);

    // Insert data
    //
    data.iter().for_each(|rec| {
        wtr.serialize(rec).unwrap();
    });

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}
