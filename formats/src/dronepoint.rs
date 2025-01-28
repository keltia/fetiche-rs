// ----- New `DronePoint`, flattened struct

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

/// Represents a flattened structure containing comprehensive information
/// related to a drone's data feed.
///
/// This struct consolidates various elements from drone-related feeds
/// (e.g., Avionix, Senhive) into a unified structure called `DronePoint`.
/// It aims to provide a common interface for handling drone telemetry data.
///
/// ## Fields
///
/// - `time` - The timestamp of the data point (UTC).
/// - `journey` - A unique identifier for a specific drone journey.
/// - `ident` - (Optional) A unique identification string for the drone.
/// - `make` - (Optional) The manufacturer of the drone.
/// - `model` - (Optional) The specific model of the drone.
/// - `uav_type` - The type of UAV (Unmanned Aerial Vehicle) represented as a `u8`
///   value. Use [`UAVType`] for meaningful enum variants.
/// - `source` - The source of the data, represented as a `u8` value. Refer to
///   [`DataSource`] enumeration for detailed source types.
/// - `latitude` - Drone's latitude position (in degrees).
/// - `longitude` - Drone's longitude position (in degrees).
/// - `altitude` - (Optional) The geodesic or true altitude of the drone (in meters).
/// - `elevation` - (Optional) The distance to the ground (in meters).
/// - `home_lat` - (Optional) The latitude of the takeoff point or home position.
/// - `home_lon` - (Optional) The longitude of the takeoff point or home position.
/// - `home_height` - (Optional) The altitude from the takeoff point.
/// - `speed` - The current velocity of the drone (in meters per second).
/// - `heading` - The current true heading of the drone (in degrees).
/// - `state` - (Optional) The current state of the drone (e.g., motor status, airborne),
///   based on values from [`VehicleStateType`].
/// - `station_name` - (Optional) The name or identifier of the detecting station
///
/// ## Usage
/// This structure is designed to be a simple interface for accessing and manipulating
/// drone telemetry data. The fields have been narrowed down to commonly used data
/// points for tracking drones across various feeds.
///
/// Example:
/// ```rust
/// use chrono::Utc;
/// use fetiche_formats::DronePoint;
///
/// let point = DronePoint {
///     time: Utc::now(),
///     journey: "12345".into(),
///     ident: Some("DRONE-001".into()),
///     make: Some("DJI".into()),
///     model: Some("Mavic Pro".into()),
///     uav_type: 2, // Corresponds to MultiRotor
///     source: 0, // Corresponds to ADS-B
///     latitude: 48.858844,
///     longitude: 2.294351,
///     altitude: Some(100.0),
///     elevation: Some(15.0),
///     home_lat: Some(48.856613),
///     home_lon: Some(2.352222),
///     home_height: Some(0.0),
///     speed: 12.5,
///     heading: 90.0,
///     state: Some(2), // Airborne
///     station_name: Some("Station XYZ".into()),
/// };
///
/// println!("{:?}", point);
/// ```
///
/// ## See Also
/// - [`VehicleStateType`]: Represents the drone's operational states such as "MotorOff" or "Airborne."
/// - [`UAVType`]: Enumeration for drone types (e.g., FixedWing, MultiRotor, etc.).
/// - [`DataSource`]: Enum to represent different data sources like ADS-B, MLAT, etc.
///
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DronePoint {
    /// timestamp
    pub time: DateTime<Utc>,
    /// Each record is part of a drone journey with a specific ID
    pub journey: String,
    /// Identifier for the drone
    pub ident: Option<String>,
    /// Maker of the drone
    pub make: Option<String>,
    /// Model of the drone
    pub model: Option<String>,
    /// UAV Type
    pub uav_type: u8,
    /// Source
    pub source: u8,
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
    /// Geodesic aka true altitude
    pub altitude: Option<f64>,
    /// Distance to ground
    pub elevation: Option<f64>,
    /// Operator lat
    pub home_lat: Option<f64>,
    /// Operator lon
    pub home_lon: Option<f64>,
    /// Altitude from takeoff point
    pub home_height: Option<f64>,
    /// Current speed
    pub speed: f64,
    /// True heading
    pub heading: f64,
    /// Vehicle state
    pub state: Option<u8>,
    /// Name of detecting point -- system.fusion_state.source_serials
    pub station_name: Option<String>,
}

/// Enumeration of drone vehicle states.
///
/// This `enum` represents various operating states a drone can be in, such as whether
/// the motors are off, motors are on, the drone is airborne, or an unknown state.
///
/// Variants:
/// - `MotorOff` - The drone's motors are turned off (value: 0).
/// - `MotorOn` - The drone's motors are turned on (value: 1).
/// - `Airborn` - The drone is airborne (value: 2).
/// - `Unknown` - The drone is in an unknown state (value: 15, default).
///
/// ## Usage
///
/// This can be used to determine the current state of a drone for telemetry or control purposes.
///
/// Example:
/// ```rust
/// use fetiche_formats::VehicleStateType;
///
/// let state = VehicleStateType::Airborn;
/// match state {
///     VehicleStateType::MotorOff => println!("Motors are off."),
///     VehicleStateType::MotorOn => println!("Motors are on."),
///     VehicleStateType::Airborn => println!("The drone is airborne."),
///     VehicleStateType::Unknown => println!("State is unknown."),
/// }
/// ```
///
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[repr(u8)]
pub enum VehicleStateType {
    MotorOff = 0,
    MotorOn = 1,
    Airborn = 2,
    #[default]
    Unknown = 15,
}

/// Enumeration of fusion types for drone data.
///
/// Fusion types are used to categorize how data from different sensors is combined
/// and processed to produce a comprehensive tracking output.
///
/// Variants:
/// - `Cooperative`: Data is coming from cooperative sources (e.g., ADS-B, Remote ID).
/// - `Surveillance`: Data is coming from surveillance sources (e.g., radar, surveillance cameras).
/// - `Both`: Data is fused from both cooperative and surveillance sources.
///
/// ## Example
/// ```rust
/// use fetiche_formats::FusionType;
///
/// let fusion_type = FusionType::Cooperative;
/// match fusion_type {
///     FusionType::Cooperative => println!("Using cooperative data."),
///     FusionType::Surveillance => println!("Using surveillance data."),
///     FusionType::Both => println!("Using both cooperative and surveillance data."),
/// }
/// ```
///
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[repr(u8)]
pub enum FusionType {
    Cooperative = 0,
    Surveillance = 1,
    Both = 2,
}

/// Enumeration of UAV (Unmanned Aerial Vehicle) types.
///
/// This `enum` represents various types of UAVs, categorized by their design and propulsion
/// systems, as well as a fallback type for unknown or undefined UAVs.
///
/// # Variants
///
/// - `Unknown`
///   Represents an unknown or undefined UAV type (default variant with value: 0).
///
/// - `FixedWing`
///   Represents fixed-wing UAVs, commonly used for long-distance operations (value: 1).
///
/// - `MultiRotor`
///   Represents multi-rotor UAVs, characterized by multiple rotors for vertical lift (value: 2).
///
/// - `Gyroplane`
///   Represents UAVs using gyroscopic propulsion for lift and navigation (value: 3).
///
/// - `HybridLift`
///   Represents hybrid-lift UAVs that combine multiple mechanisms for lift (value: 4).
///
/// - `Other`
///   Represents any other UAV type not covered in the defined variants (value: 15).
///
/// # Example
///
/// ```rust
/// use fetiche_formats::UAVType;
///
/// let uav = UAVType::MultiRotor;
/// match uav {
///     UAVType::Unknown => println!("Unknown type"),
///     UAVType::FixedWing => println!("Fixed-wing UAV"),
///     UAVType::MultiRotor => println!("Multi-rotor UAV"),
///     UAVType::Gyroplane => println!("Gyroplane UAV"),
///     UAVType::HybridLift => println!("Hybrid-lift UAV"),
///     UAVType::Other => println!("Other UAV type"),
/// }
/// ```
///
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[repr(u8)]
pub enum UAVType {
    #[default]
    Unknown = 0,
    FixedWing = 1,
    MultiRotor = 2,
    Gyroplane = 3,
    HybridLift = 4,
    Other = 15,
}

/// Enumeration of data sources for drone tracking.
///
/// This `enum` represents various types of data sources that can be used to track drones.
/// These include technologies and communication methods, each represented as a variant.
///
/// Variants:
/// - `A` - ADS-B: Automatic Dependent Surveillance–Broadcast
/// - `M` - MLAT: Multilateration
/// - `U` - UAT: Universal Access Transceiver
/// - `L` - ADS-L: Low-power ADS-B
/// - `F` - FLARM: Flight Alarm System
/// - `O` - OGN: Open Glider Network
/// - `Rid` - Remote-ID: Drone identification via Remote-ID
/// - `Lte` - 4G/5G: Communication over cellular networks
/// - `P` - PilotAware: Low-cost traffic detection
/// - `N` - FANET: Flying Ad-hoc Network
/// - `X` - Asterix: Standardized Eurocontrol surveillance format
///
/// The enum derives traits that allow serialization, deserialization, and case-insensitive
/// name matching using the `strum` crate.
///
/// Example:
/// ```rust
/// use std::str::FromStr;
/// use fetiche_formats::DataSource;
///
/// let source = DataSource::Rid;
/// let source_as_u8: u8 = source.into();
/// assert_eq!(source_as_u8, 6);
///
/// let source_from_str = DataSource::from_str("Rid").expect("Invalid source");
/// assert_eq!(source_from_str, DataSource::Rid);
/// ```
///
/// Additionally, this enum provides a utility to map strings directly to their corresponding
/// `u8` value without needing to construct a `DataSource` instance:
///
/// ```
/// use fetiche_formats::DataSource;
///
/// let source_value = DataSource::str_to_source("Rid");
/// assert_eq!(source_value, 6);
/// let unknown_source = DataSource::str_to_source("UNKNOWN");
/// assert_eq!(unknown_source, 255);
/// ```
///
/// # Note:
/// The fallback value for invalid mappings is `255`, which represents an undefined source.
///
#[derive(Debug, Deserialize, Serialize, strum::Display, EnumString, strum::VariantNames, PartialEq)]
#[strum(serialize_all = "UPPERCASE")]
pub enum DataSource {
    /// ADS-B
    A,
    /// MLAT
    M,
    /// UAT,
    U,
    /// ADS-L
    L,
    /// FLARM
    F,
    /// OGN
    O,
    /// Remote-ID
    Rid,
    /// 4G/5G
    Lte,
    /// PilotAware
    P,
    /// FANET
    N,
    /// Asterix
    X,
}

impl From<DataSource> for u8 {
    fn from(value: DataSource) -> Self {
        match value {
            DataSource::A => 0,
            DataSource::M => 1,
            DataSource::U => 2,
            DataSource::L => 3,
            DataSource::F => 4,
            DataSource::O => 5,
            DataSource::Rid => 6,
            DataSource::Lte => 7,
            DataSource::P => 8,
            DataSource::N => 9,
            DataSource::X => 10,
        }
    }
}

impl DataSource {
    /// Direct mapping between a string and the u8 value as a source.
    ///
    pub fn str_to_source(value: &str) -> u8 {
        match value {
            "A" => 0,
            "M" => 1,
            "U" => 2,
            "L" => 3,
            "F" => 4,
            "O" => 5,
            "Rid" => 6,
            "Lte" => 7,
            "P" => 8,
            "N" => 9,
            "X" => 10,
            _ => 255,
        }
    }
}
