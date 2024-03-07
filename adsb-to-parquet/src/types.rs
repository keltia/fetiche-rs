use serde::{Deserialize, Serialize};

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Cat21 {
    #[serde(rename = "020.EmitterCategory")]
    pub ecat: u8,
    #[serde(rename = "040.GBS")]
    pub gbs: u8,
    #[serde(rename = "070.ModeA")]
    pub mode3a: String,
    #[serde(rename = "073.TimeRecPosition")]
    pub time_rec_position: f32,
    #[serde(rename = "080.AircraftAddress")]
    pub aircraft_addr: String,
    #[serde(rename = "131.Latitude")]
    pub latitude: f32,
    #[serde(rename = "131.Longitude")]
    pub longitude: f32,
    #[serde(rename = "140.GeometricAltitude")]
    pub geometric_altitude: f32,
    #[serde(rename = "145.FlightLevel")]
    pub flight_level: f32,
    #[serde(rename = "155.BarometricVerticalRate")]
    pub barometric_vertical_rate: f32,
    #[serde(rename = "157.RE")]
    pub re: Option<String>,
    #[serde(rename = "157.GeometricVerticalRate")]
    pub geometric_vertical_rate: f32,
    #[serde(rename = "160.GroundSpeed")]
    pub ground_speed: f32,
    #[serde(rename = "160.TrackAngle")]
    pub track_angle: f32,
    #[serde(rename = "170.Callsign")]
    pub callsign: String,
    #[serde(rename = "R")]
    pub r: Option<SGV>,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Deserialize, Serialize)]
pub struct SGV {
    #[serde(rename = "STP")]
    pub stp: Option<String>,
    #[serde(rename = "HTS")]
    pub hts: Option<String>,
    #[serde(rename = "HTT")]
    pub htt: Option<String>,
    #[serde(rename = "HRD")]
    pub hrd: Option<String>,
    #[serde(rename = "GSS")]
    pub gss: Option<String>,
    #[serde(rename = "HGT")]
    pub hgt: Option<String>,
}
