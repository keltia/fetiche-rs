use chrono::{DateTime, Utc};

use crate::Asd;

#[derive(Debug, Deserialize, InfluxDbWriteable, Serialize)]
pub struct Drone {
    pub time: DateTime<Utc>,
    // Each record is part of a drone journey with a specific ID
    #[influxdb(tag)]
    pub journey: u32,
    // Identifier for the drone
    pub ident: String,
    // Model of the drone
    pub model: Option<String>,
    // Source ([see src/site/asd.rs]) of the data
    pub source: String,
    // Point/record ID
    pub location: u32,
    // $7 (actually f32)
    pub latitude: f32,
    // $8 (actually f32)
    pub longitude: f32,
    // Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    // Distance to ground (estimated every 15s)
    pub elevation: Option<u32>,
    // $13 (actually f32)
    pub home_lat: Option<f32>,
    // $14 (actually f32)
    pub home_lon: Option<f32>,
    // Altitude from takeoff point
    pub home_height: Option<f32>,
    // Current speed
    pub speed: f32,
    // True heading
    pub heading: f32,
    // Name of detecting point
    #[influxdb(tag)]
    pub station_name: Option<String>,
    // Latitude (actually f32)
    pub station_lat: Option<f32>,
    // Longitude (actually f32)
    pub station_lon: Option<f32>,
}

impl From<Asd> for Drone {
    fn from(value: Asd) -> Self {
        let tod = DateTime::<Utc>::(&value.timestamp, "%Y-%m-%d %H:%M:%S");

        Drone {
            time: tod,
            journey: value.journey,
            ident: value.ident.clone(),
            model: value.model.clone(),
            source: value.source.clone(),
            location: value.location,
            latitude: value.latitude.parse::<f32>().unwrap(),
            longitude: value.longitude.parse::<f32>().unwrap(),
            altitude: value.altitude,
            elevation: value.elevation,
            home_lat: Some(value.home_lat.unwrap().parse::<f32>().unwrap()),
            home_lon: Some(value.home_lon.unwrap().parse::<f32>().unwrap()),
            home_height: Some(value.home_height.unwrap()),
            speed: value.speed,
            heading: value.heading,
            station_name: Some(value.station_name.unwrap()),
            station_lat: Some(value.station_lat.unwrap().parse::<f32>().unwrap()),
            station_lon: Some(value.station_lon.unwrap().parse::<f32>().unwrap()),
        }
    }
}
