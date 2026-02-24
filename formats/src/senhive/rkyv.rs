use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use super::{
    Altitudes, Coordinates, FusedData, FusedValue, FusionState, Location, PilotIdentification,
    PilotState, System, TSLog, VehicleIdentification, VehicleState,
};

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RFusedData {
    pub version: String,
    pub system: RSystem,
    pub vehicle_identification: RVehicleIdentification,
    pub vehicle_state: RVehicleState,
    pub pilot_identification: Option<RPilotIdentification>,
    pub pilot_state: RPilotState,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RSystem {
    pub track_id: String,
    pub timestamp_millis: i64,
    pub timestamp_log: Option<Vec<RTSLog>>,
    pub fusion_state: RFusionState,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RTSLog {
    pub process_name: String,
    pub timestamp_millis: i64,
    pub msg: Option<String>,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RFusionState {
    pub fusion_type: u8,
    pub source_serials: Vec<String>,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RVehicleState {
    pub location: RLocation,
    pub altitudes: RAltitudes,
    pub ground_speed: Option<RFusedValue>,
    pub vertical_speed: Option<RFusedValue>,
    pub orientation: Option<RFusedValue>,
    pub state: Option<u8>,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RVehicleIdentification {
    pub serial: Option<String>,
    pub mac: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub uav_type: u8,
}

#[derive(Archive, Clone, Copy, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RFusedValue {
    pub value: f64,
    pub uncertainty: Option<f64>,
}

impl From<RFusedValue> for f64 {
    /// Easy conversion into plain f64
    fn from(fv: RFusedValue) -> Self {
        fv.value
    }
}

impl Default for RFusedValue {
    fn default() -> Self {
        Self {
            value: 0.0,
            uncertainty: None,
        }
    }
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RAltitudes {
    /// Above take-off location [m]
    pub ato: Option<RFusedValue>,
    /// Above ground level [m]
    pub agl: Option<RFusedValue>,
    /// Above mean sea level [m]
    pub amsl: Option<RFusedValue>,
    /// Real geodetic altitude.
    pub geodetic: Option<RFusedValue>,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RLocation {
    pub coordinates: RCoordinates,
    pub uncertainty: Option<f64>,
    /// This is a string with a 7-point WKT Polygon
    pub likelihood: Option<String>,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RPilotState {
    pub location: RLocation,
    pub location_type: u8,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RPilotIdentification {
    pub id: u64,
    pub name: String,
    pub location: Option<RLocation>,
}

#[derive(Archive, RkyvDeserialize, RkyvSerialize, Debug, PartialEq)]
pub struct RCoordinates {
    pub lon: f64,
    pub lat: f64,
}

impl From<&FusedData> for RFusedData {
    fn from(value: &FusedData) -> Self {
        Self {
            version: value.version.clone(),
            system: RSystem::from(&value.system),
            vehicle_identification: RVehicleIdentification::from(&value.vehicle_identification),
            vehicle_state: RVehicleState::from(&value.vehicle_state),
            pilot_identification: value
                .pilot_identification
                .as_ref()
                .map(RPilotIdentification::from),
            pilot_state: RPilotState::from(&value.pilot_state),
        }
    }
}

impl From<&System> for RSystem {
    fn from(value: &System) -> Self {
        Self {
            track_id: value.track_id.clone(),
            timestamp_millis: value.timestamp.timestamp_millis(),
            timestamp_log: value.timestamp_log.as_ref().map(|items| {
                items
                    .iter()
                    .map(RTSLog::from)
                    .collect::<Vec<RTSLog>>()
            }),
            fusion_state: RFusionState::from(&value.fusion_state),
        }
    }
}

impl From<&TSLog> for RTSLog {
    fn from(value: &TSLog) -> Self {
        Self {
            process_name: value.process_name.clone(),
            timestamp_millis: value.timestamp.timestamp_millis(),
            msg: value.msg.clone(),
        }
    }
}

impl From<&FusionState> for RFusionState {
    fn from(value: &FusionState) -> Self {
        Self {
            fusion_type: value.fusion_type,
            source_serials: value.source_serials.clone(),
        }
    }
}

impl From<&VehicleState> for RVehicleState {
    fn from(value: &VehicleState) -> Self {
        Self {
            location: RLocation::from(&value.location),
            altitudes: RAltitudes::from(&value.altitudes),
            ground_speed: value.ground_speed.map(RFusedValue::from),
            vertical_speed: value.vertical_speed.map(RFusedValue::from),
            orientation: value.orientation.map(RFusedValue::from),
            state: value.state,
        }
    }
}

impl From<&VehicleIdentification> for RVehicleIdentification {
    fn from(value: &VehicleIdentification) -> Self {
        Self {
            serial: value.serial.clone(),
            mac: value.mac.clone(),
            make: value.make.clone(),
            model: value.model.clone(),
            uav_type: value.uav_type,
        }
    }
}

impl From<FusedValue> for RFusedValue {
    fn from(value: FusedValue) -> Self {
        Self {
            value: value.value,
            uncertainty: value.uncertainty,
        }
    }
}

impl From<&Altitudes> for RAltitudes {
    fn from(value: &Altitudes) -> Self {
        Self {
            ato: value.ato.map(RFusedValue::from),
            agl: value.agl.map(RFusedValue::from),
            amsl: value.amsl.map(RFusedValue::from),
            geodetic: value.geodetic.map(RFusedValue::from),
        }
    }
}

impl From<&Location> for RLocation {
    fn from(value: &Location) -> Self {
        Self {
            coordinates: RCoordinates::from(&value.coordinates),
            uncertainty: value.uncertainty,
            likelihood: value.likelihood.clone(),
        }
    }
}

impl From<&PilotState> for RPilotState {
    fn from(value: &PilotState) -> Self {
        Self {
            location: RLocation::from(&value.location),
            location_type: value.location_type,
        }
    }
}

impl From<&PilotIdentification> for RPilotIdentification {
    fn from(value: &PilotIdentification) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            location: value.location.as_ref().map(RLocation::from),
        }
    }
}

impl From<&Coordinates> for RCoordinates {
    fn from(value: &Coordinates) -> Self {
        Self {
            lon: value.lon,
            lat: value.lat,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RFusedData;
    use crate::senhive::FusedData;

    const SAMPLE: &str = r##"
{"version":"1.0.0","system":{"trackID":"561c2855-d4a2-4109-aada-56ef79c00ffe","timestamp":"2024-10-23T13:02:03+00:00","timestampLog":[],"fusionState":{"fusionType":1,"sourceSerials":["1424823000354"]}},"vehicleIdentification":{"serial":"F5YHX23CR0030UT5","mac":null,"make":"DJI","model":"DJI Mini 3","uavType":2},"vehicleState":{"location":{"coordinates":{"lon":2.378793716430664,"lat":48.57561492919922},"uncertainty":null,"likelihood":"POLYGON ((2.3789294904782423 48.57577052279335, 2.379065263690095 48.57561492887997, 2.378929489642521 48.575459335445466, 2.3786579432188075 48.575459335445466, 2.3785221691712337 48.57561492887997, 2.3786579423830863 48.57577052279335, 2.3789294904782423 48.57577052279335))"},"altitudes":{"ato":{"value":106.5,"uncertainty":null},"agl":{"value":106.5,"uncertainty":null},"amsl":null,"geodetic":{"value":236.0,"uncertainty":null}},"groundSpeed":{"value":0.022360679774997897,"uncertainty":null},"verticalSpeed":null,"orientation":{"value":153.0,"uncertainty":null},"state":2},"pilotIdentification":null,"pilotState":{"location":{"coordinates":{"lon":2.378805160522461,"lat":48.57560348510742},"uncertainty":null,"likelihood":null},"locationType":2}}
"##;

    #[test]
    fn rkyv_roundtrip_fused_data() {
        let data: FusedData = serde_json::from_str(SAMPLE).unwrap();
        let data: RFusedData = (&data).into();

        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data).unwrap();
        let decoded =
            rkyv::from_bytes::<RFusedData, rkyv::rancor::Error>(&bytes).unwrap();

        assert_eq!(data, decoded);
    }
}
