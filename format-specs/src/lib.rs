//! Definition of a data format-specs
//!
//! This module makes the link between the shared output format-specs `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new format-specs, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input format-specs and the transformations needed.
//!

pub mod aeroscope;
pub mod asd;
pub mod opensky;
pub mod safesky;

use crate::aeroscope::Aeroscope;
use crate::asd::Asd;
use crate::safesky::Safesky;

use anyhow::Result;
use csv::{Reader, WriterBuilder};
use log::{debug, trace};
use serde::{Deserialize, Serialize};

use std::fmt::{Debug, Display, Formatter};
use std::io::Read;

#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Format {
    #[default]
    None,
    Aeroscope,
    Asd,
    Opensky,
    Safesky,
}

/// Macro to create the code which deserialize known types.
///
/// It takes three arguments:
/// - from
/// - object
/// - list of types
///
macro_rules! into_cat21 {
    ($from: ident, $rec:ident, $($name:ident),+) => {
        match $from {
        $(
            Format::$name => {
                let l: $name = $rec.deserialize(None).unwrap();
                Cat21::from(&l)
            },
        )+
            _ => panic!("unknown format"),
        }
    };
}

impl Format {
    // Process each record coming from the input source, apply `Cat::from()` onto it
    // and return the list.  This is used when reading from the csv files.
    //
    pub fn from_csv<T>(self, rdr: &mut Reader<T>) -> Result<Vec<Cat21>>
    where
        T: Read,
    {
        debug!("Reading & transforming…");
        let res: Vec<_> = rdr
            .records()
            .enumerate()
            .map(|(cnt, rec)| {
                let rec = rec.unwrap();
                debug!("rec={:?}", rec);
                let mut line = into_cat21!(self, rec, Aeroscope, Asd, Safesky);
                line.rec_num = cnt;
                line
            })
            .collect();
        Ok(res)
    }
}

impl From<&str> for Format {
    /// Create a format-specs from its name
    ///
    fn from(s: &str) -> Self {
        match s {
            "aeroscope" => Format::Aeroscope,
            "asd" => Format::Asd,
            "opensky" => Format::Opensky,
            "safesky" => Format::Safesky,
            _ => Format::None,
        }
    }
}

impl Display for Format {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s: String = match self {
            Format::Aeroscope => "aeroscope".into(),
            Format::Asd => "asd".into(),
            Format::Safesky => "safesky".into(),
            Format::Opensky => "opensky".into(),
            Format::None => "none".into(),
        };
        write!(f, "{}", s)
    }
}

/// Default SAC: France
const DEF_SAC: usize = 8;
/// Default SIC
const DEF_SIC: usize = 200;

/// This structure hold a general location object with lat/long.
///
/// In CSV files, the two fields are merged into this struct on deserialization
/// and used as-is when coming from JSON.
///
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Position {
    // Latitude in degrees
    pub latitude: f32,
    /// Longitude in degrees
    pub longitude: f32,
}

impl Default for Position {
    /// makes testing easier
    #[inline]
    fn default() -> Self {
        Position {
            latitude: 0.0,
            longitude: 0.0,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TodCalculated {
    C,
    L,
    N,
    R,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Bool {
    Y,
    N,
}

/// Convert into feet
///
#[inline]
pub fn to_feet(a: f32) -> u32 {
    (a * 3.28084) as u32
}

/// Convert into knots
///
#[inline]
pub fn to_knots(a: f32) -> f32 {
    a * 0.54
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
/// As the number implies (> 127), it is created to describe a special Civil/Military category,
/// specialised for drones.
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
pub fn prepare_csv<T>(data: Vec<T>) -> Result<String>
where
    T: Serialize,
{
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_default() {
        let s = Format::default();

        assert_eq!(Format::None, s);
    }

    #[test]
    fn test_to_feet() {
        assert_eq!(1, to_feet(0.305))
    }

    #[test]
    fn test_to_knots() {
        assert_eq!(1.00008, to_knots(1.852))
    }
}
