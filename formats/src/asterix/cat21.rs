use crate::{Bool, TodCalculated, DEF_SAC, DEF_SIC};

use serde::Serialize;

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
            sac: DEF_SAC,
            sic: DEF_SIC,
            alt_geo_ft: 0,
            pos_lat_deg: 0.0,
            pos_long_deg: 0.0,
            alt_baro_ft: 0,
            tod: 0,
            rec_time_posix: 0,
            rec_time_ms: 0,
            emitter_category: 0,
            differential_correction: Bool::default(),
            ground_bit: Bool::default(),
            simulated_target: Bool::default(),
            test_target: Bool::default(),
            from_ft: Bool::default(),
            selected_alt_capability: Bool::default(),
            spi: Bool::default(),
            link_technology_cddi: Bool::default(),
            link_technology_mds: Bool::default(),
            link_technology_uat: Bool::default(),
            link_technology_vdl: Bool::default(),
            link_technology_other: Bool::default(),
            descriptor_atp: 0,
            alt_reporting_capability_ft: 0,
            target_addr: 0,
            cat: 0,
            line_id: 0,
            ds_id: 0,
            report_type: 0,
            tod_calculated: TodCalculated::default(),
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
