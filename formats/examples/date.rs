use chrono::NaiveDateTime;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asd {
    /// Hidden UNIX timestamp
    #[serde(skip_deserializing)]
    pub tm: i64,
    /// Each record is part of a drone journey with a specific ID
    pub journey: u32,
    /// Identifier for the drone
    pub ident: String,
    /// Model of the drone
    pub model: Option<String>,
    /// Source ([see src/site/asd.rs]) of the data
    pub source: String,
    /// Point/record ID
    pub location: u32,
    /// Date of event (in the non standard YYYY-MM-DD HH:MM:SS formats)
    pub timestamp: String,
    /// $7 (actually f32)
    #[serde_as(as = "DisplayFromStr")]
    pub latitude: f32,
    /// $8 (actually f32)
    #[serde_as(as = "DisplayFromStr")]
    pub longitude: f32,
    /// Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    /// Distance to ground (estimated every 15s)
    pub elevation: Option<i32>,
    /// Undocumented
    pub gps: Option<u32>,
    /// Signal level (in dB)
    pub rssi: Option<i32>,
    /// $13 (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub home_lat: Option<f32>,
    /// $14 (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub home_lon: Option<f32>,
    /// Altitude from takeoff point
    pub home_height: Option<f32>,
    /// Current speed
    pub speed: f32,
    /// True heading
    pub heading: f32,
    /// Name of detecting point
    pub station_name: Option<String>,
    /// Latitude (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub station_latitude: Option<f32>,
    /// Longitude (actually f32)
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub station_longitude: Option<f32>,
}

fn fix_tm(inp: Asd) -> Result<Asd> {
    let tod = NaiveDateTime::parse_from_str(&inp.timestamp, "%Y-%m-%d %H:%M:%S")?.timestamp();
    let mut out = inp.clone();
    out.tm = tod;
    Ok(out)
}

fn main() -> Result<()> {
    let input = r##"{"journey":36354,"ident":"08RDE92001041T","model":"MavicPro","source":"as","location":1674998,"timestamp":"2023-10-03 01:00:52","latitude":"44.862114","longitude":"-0.523638","altitude":120,"elevation":63,"gps":null,"rssi":null,"home_lat":"44.862034","home_lon":"-0.523769","home_height":59,"speed":2,"heading":50,"station_name":"0QRDKC2R038370","station_latitude":"44.831462","station_longitude":"-0.702713"}
    "##;

    let res: Asd = serde_json::from_str(input)?;
    let res = fix_tm(res)?;
    println!("tm={}", res.tm);
    println!("timestamp={}", res.timestamp);

    Ok(())
}
