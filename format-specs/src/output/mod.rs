pub mod aeroscope;
pub mod asd;
pub mod opensky;
pub mod safesky;

use anyhow::Result;
use chrono::NaiveDateTime;
use csv::WriterBuilder;
use log::{debug, trace};
use serde::Serialize;

use crate::{Bool, Position, TodCalculated};

/// Our pseudo cat21 csv output, we add the mapping from the awk script in comment
///
/// SAC:SIC:ALT_GEO_FT:POS_LAT_DEG:POS_LONG_DEG:ALT_BARO_FT:TOD:REC_TIME_POSIX:REC_TIME_MS:
/// EMITTER_CATEGORY:DIFFERENTIAL_CORRECTION:GROUND_BIT:SIMULATED_TARGET:TEST_TARGET:FROM_FFT:
/// SELECTED_ALT_CAPABILITY:SPI:LINK_TECHNOLOGY_CDTI:LINK_TECHNOLOGY_MDS:LINK_TECHNOLOGY_UAT:
/// LINK_TECHNOLOGY_VDL:LINK_TECHNOLOGY_OTHER:DESCRIPTOR_ATP:ALT_REPORTING_CAPABILITY_FT:
/// TARGET_ADDR:CAT:LINE_ID:DS_ID:REPORT_TYPE:TOD_CALCULATED:CALLSIGN:GROUNDSPEED_KT:T
/// RACK_ANGLE_DEG:REC_NUM
///
/// Time calculations are done in `i64` to avoid the upcoming 2037 bug with 32-bit time_t.
/// Most systems are using `i64` now.
///
/// XXX most of the data is fictive in order to fill all the fields.  Data generated from UAS
/// records are not as complete as Cat21 data from ADS-B or MODE-S sources can be.
/// See Cat129 below for UAS specific format.
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
    pub tod: i64,
    // $d1
    pub rec_time_posix: i64,
    // $d2
    pub rec_time_ms: u32,
    // $e
    pub emitter_category: usize,
    // $f
    pub differential_correction: Bool,
    // $g
    pub ground_bit: Bool,
    // $h
    pub simulated_target: Bool,
    // $i
    pub test_target: Bool,
    // $j
    pub from_ft: Bool,
    // $k
    pub selected_alt_capability: Bool,
    // $l
    pub spi: Bool,
    // $l1 (these ought to be an enum)
    pub link_technology_cddi: Bool,
    // $l2
    pub link_technology_mds: Bool,
    // $l3
    pub link_technology_uat: Bool,
    // $l4
    pub link_technology_vdl: Bool,
    // $l5
    pub link_technology_other: Bool,
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
    pub tod_calculated: TodCalculated,
    // $u
    pub callsign: String,
    // $v
    pub groundspeed_kt: f32,
    // $w
    pub track_angle_deg: f32,
    // $y
    pub rec_num: usize,
}

impl Default for Cat21 {
    /// Invalid default
    ///
    fn default() -> Self {
        Cat21 {
            sac: 0,
            sic: 0,
            alt_geo_ft: 0,
            pos_lat_deg: 0.0,
            pos_long_deg: 0.0,
            alt_baro_ft: 0,
            tod: 0,
            rec_time_posix: 0,
            rec_time_ms: 0,
            emitter_category: 0,
            differential_correction: Bool::N,
            ground_bit: Bool::N,
            simulated_target: Bool::N,
            test_target: Bool::N,
            from_ft: Bool::N,
            selected_alt_capability: Bool::N,
            spi: Bool::N,
            link_technology_cddi: Bool::N,
            link_technology_mds: Bool::N,
            link_technology_uat: Bool::N,
            link_technology_vdl: Bool::N,
            link_technology_other: Bool::N,
            descriptor_atp: 0,
            alt_reporting_capability_ft: 0,
            target_addr: 0,
            cat: 0,
            line_id: 0,
            ds_id: 0,
            report_type: 0,
            tod_calculated: TodCalculated::N,
            callsign: "".to_string(),
            groundspeed_kt: 0.0,
            track_angle_deg: 0.0,
            rec_num: 0,
        }
    }
}

impl Cat21 {
    pub fn error(e: &str) -> Self {
        Cat21 {
            rec_num: 0,
            callsign: e.to_owned(),
            ..Default::default()
        }
    }
}

/// Cat129 is a special UAS-specific category defined in 2019.
///
/// See: https://www.eurocontrol.int/sites/default/files/2019-06/cat129p29ed12_0.pdf
///
#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Cat129 {
    /// Source Identification
    sac: usize,
    sic: usize,
    /// Destination Identification (more or less the operator?)
    dac: usize,
    dic: usize,
    /// Manufacturer Identification
    uas_manufacturer_id: String,
    uas_model_id: String,
    uas_serial: String,
    uas_reg_country: String,
    /// Aeronautical data
    /// tod is number of 1/128s since Midnight
    /// Example:
    /// ```
    /// # use chrono::NaiveDateTime;
    ///
    /// let tod = NaiveDateTime::parse_from_str(&line.timestamp, "%Y-%m-%d %H:%M:%S")
    ///            .unwrap()
    ///            .timestamp();
    /// let tod = 128 * (tod % 86400);
    /// ```
    tod: i64,
    position: Position,
    alt_sea_lvl: f32,
    alt_gnd_lvl: f32,
    gnss_acc: f32,
    ground_speed: f32,
    vert_speed: f32,
}

/// Output the final csv file with a different delimiter 'now ":")
///
pub fn prepare_csv<T>(data: Vec<T>) -> Result<String> {
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
