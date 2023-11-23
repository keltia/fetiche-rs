use chrono::{DateTime, Utc};
use influxdb::{InfluxDbWriteable, Timestamp};
use parquet_derive::ParquetRecordWriter;
use serde::{Deserialize, Serialize};

/// This is derived from the Asd structure for convenience.
///
/// We fix the obvious issues with timestamp being in a non-standard format and
/// geolocation being strings instead of floats, time being a proper type instead
/// of a string, etc.
///
/// We do not have to convert to Asterix specificities like non standard altitude
/// and non-metric units.
///
/// `time` is a DateTime to help insertion in a time-series db like InfluxDB.
///
#[derive(Clone, Debug, Deserialize, InfluxDbWriteable, Serialize)]
pub struct DronePoint {
    /// UNIX timestamp
    pub time: DateTime<Utc>,
    /// Each record is part of a drone journey with a specific ID
    #[influxdb(tag)]
    pub journey: u32,
    /// Identifier for the drone
    #[influxdb(tag)]
    pub drone_id: String,
    /// Model of the drone
    #[influxdb(tag)]
    pub model: Option<String>,
    /// Source ([see src/site/asd.rs]) of the data
    #[influxdb(tag)]
    pub source: String,
    /// Monotonically increasing ID == PointID
    pub location: u32,
    /// Actual position (lat)
    pub latitude: f32,
    /// Actual position (lat)
    pub longitude: f32,
    /// Altitude, can be either null or negative (?)
    pub altitude: Option<i16>,
    /// Distance to ground (estimated every 15s)
    pub elevation: Option<i32>,
    /// $13 (actually f32)
    pub home_lat: Option<f32>,
    /// $13 (actually f32)
    pub home_lon: Option<f32>,
    /// Altitude from takeoff point
    pub home_height: Option<f32>,
    /// Current speed
    pub speed: f32,
    /// True heading
    pub heading: f32,
    /// Name of detecting point
    #[influxdb(tag)]
    pub station_name: Option<String>,
    /// Station location
    pub station_lat: Option<f32>,
    /// Station location
    pub station_lon: Option<f32>,
}

/// A journey is a state vectors: a vector of the measured 3D points with a timestamp.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Journey {
    /// Journey ID
    pub id: u32,
    /// All the points
    pub points: Vec<DronePoint>,
}

impl Journey {
    // Write a journey into a specific query/table
    //
    // pub async fn write(&self, client: &influxdb::Client) -> Result<String, influxdb::Error> {
    //     let payload = self.points.iter().map(|p| p.into_query(self.id));
    //     client.query(payload).await
    // }
}
