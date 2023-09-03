use serde::Serialize;

/// Our pseudo cat21 for ADS-B csv output, we add the mapping from the awk script in comment
///
/// REC_TIME_POSIX:TOD:TARGET_ADDR:CALLSIGN:POS_LAT_DEG:POS_LONG_DEG:ALT_GEO_FT
///
/// Time calculations are done in `i64` to avoid the upcoming 2037 bug with 32-bit time_t.
/// Most systems are using `i64` now.
///
/// XXX most of the data is fictive in order to fill all the fields.  Data generated from UAS
/// records are not as complete as Cat21 data from ADS-B or MODE-S sources can be.
/// See Cat129 below for UAS specific format.
///
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Adsb21 {
    pub rec_time_posix: i64,
    pub tod: i64,
    pub target_addr: u32,
    pub callsign: String,
    pub pos_lat_deg: f32,
    pub pos_long_deg: f32,
    pub alt_geo_ft: u32,
}

impl Adsb21 {
    pub fn error(e: &str) -> Self {
        Adsb21 {
            callsign: e.to_owned(),
            ..Default::default()
        }
    }
}
