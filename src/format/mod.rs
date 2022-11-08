//! Definition of a data format
//!
//! This module makes the link between the shared output format `Cat21` and the different
//! input formats defined in the other modules.
//!
//! To add a new format, insert here the different hooks (`Source`, etc.) & names and a `FORMAT.rs`
//! file which will define the input format and the transformations needed.
//!

pub mod aeroscope;
pub mod asd;
pub mod safesky;

use crate::format::aeroscope::Aeroscope;
use crate::format::asd::Asd;
use crate::format::safesky::Safesky;
use std::fmt::{Display, Formatter};

use anyhow::Result;
use csv::{Reader, WriterBuilder};
use log::trace;
use serde::{Deserialize, Serialize};

use std::io::Read;

#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Format {
    None,
    Aeroscope,
    Asd,
    Safesky,
}

impl Default for Format {
    fn default() -> Self {
        Format::None
    }
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
            Source::$name => {
                let l: $name = $rec.deserialize(None).unwrap();
                Cat21::from(l)
            },
        )+
            _ => panic!("unknown format"),
        }
    };
}

impl Format {
    // Process each record coming from the input source, apply `Cat::from()` onto it
    // and return the list.
    //
    pub fn process<T>(self, rdr: &mut Reader<T>) -> Result<Vec<Cat21>>
    where
        T: Read,
    {
        trace!("Reading & transforming…");
        let res: Vec<_> = rdr
            .records()
            .enumerate()
            .map(|(cnt, rec)| {
                let rec = rec.unwrap();
                trace!("rec={:?}", rec);
                let mut line = into_cat21!(self, rec, Aeroscope, Asd, Safesky);
                line.rec_num = cnt;
                line
            })
            .collect();
        Ok(res)
    }
}

impl From<&str> for Format {
    /// Create a format from its name
    ///
    fn from(s: &str) -> Self {
        match s {
            "aeroscope" => Format::Aeroscope,
            "asd" => Format::Asd,
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
            Format::None => "none".into(),
        };
        write!(f, "{}", s)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_default() {
        let s = Format::new();

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
