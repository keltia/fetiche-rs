use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serializer};

pub fn now_seconds() -> i64 {
    Timestamp::now().as_second()
}

pub mod serde_rfc3339 {
    use super::*;

    pub fn serialize<S>(value: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<Timestamp>()
            .map_err(serde::de::Error::custom)
    }
}
