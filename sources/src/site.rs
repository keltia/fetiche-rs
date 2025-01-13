//!  Module that defines what is a site (website, API endpoint, etc.)
//!
//! This is used to configure the list of possible sources through `sources.hcl`.
//!
//! Sites can have different ways to authenticate (or not) the request, some require to
//! fetch a token first, some use an API key directly.
//!
//! You can define a set of possible routes for a site depending on how the API/site is
//! designed.
//!
//! History:

use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use fetiche_formats::Format;

use crate::{AsyncStreamable, Auth, Capability, Fetchable, Routes, Sources, Streamable};

/// Describe what a site is, its capabilities, access methods and authentication method.
///
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Site {
    /// Features of the site
    pub features: Vec<Capability>,
    /// Which data are we getting (drone or plain ads-b)
    #[serde(rename = "type")]
    pub dtype: DataType,
    /// Name of the site
    #[serde(skip_deserializing)]
    pub name: String,
    /// Storage path for tokens
    #[serde(skip_deserializing)]
    pub token_base: PathBuf,
    /// Type of input
    pub format: String,
    /// Base URL (to avoid repeating)
    pub base_url: String,
    /// Credentials (will be empty by default and will get filled in by clients)
    pub auth: Option<Auth>,
    /// Different URLs available
    pub routes: Option<Routes>,
}

/// Define the kind of data the source is managing
///
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    /// Plain ADS-B traffic
    Adsb,
    /// Drone specific traffic
    Drone,
    /// Invalid datatype
    #[default]
    Invalid,
}

impl From<&str> for DataType {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        match value.as_str() {
            "adsb" => DataType::Adsb,
            "drone" => DataType::Drone,
            _ => DataType::Invalid,
        }
    }
}

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DataType::Adsb => "adsb",
                DataType::Drone => "drone",
                DataType::Invalid => "none",
            }
        )
    }
}

impl Site {
    /// Basic `new()`
    ///
    #[tracing::instrument]
    pub fn new() -> Self {
        Site::default()
    }

    /// Return whether a site is streamable
    ///
    pub fn is_streamable(&self) -> bool {
        self.features.contains(&Capability::Stream)
    }

    /// Return the site name
    ///
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return the site formats
    ///
    pub fn format(&self) -> Format {
        Format::from_str(&self.format).unwrap()
    }

    /// Return the list of routes
    ///
    #[tracing::instrument]
    pub fn list(&self) -> Vec<&String> {
        match &self.routes {
            Some(routes) => routes.keys().collect::<Vec<_>>(),
            _ => vec![],
        }
    }

    /// Check whether site has the mentioned route
    ///
    #[tracing::instrument]
    pub fn has(&self, meth: &str) -> bool {
        match &self.routes {
            Some(routes) => routes.contains_key(meth),
            _ => false,
        }
    }

    /// Retrieve a route
    ///
    #[tracing::instrument]
    pub fn route(&self, key: &str) -> Option<&String> {
        match &self.routes {
            Some(routes) => routes.get(key),
            _ => None,
        }
    }

    /// Getter for dtype
    ///
    pub fn data(self) -> DataType {
        self.dtype
    }
}

impl Display for Site {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let auth = match self.auth.clone() {
            Some(auth) => auth,
            _ => Auth::Anon,
        };
        write!(
            f,
            "{{ format={} url={} auth={} routes={:?} }}",
            self.format, self.base_url, auth, self.routes
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rstest::rstest;

    use super::*;

    fn set_default() -> Sources {
        let cn = PathBuf::from("src").join("sources.hcl");
        assert!(cn.try_exists().is_ok());

        let cfg = Sources::new();
        dbg!(&cfg);
        assert!(cfg.is_ok());

        let cfg = cfg.unwrap();
        dbg!(&cfg);
        assert!(!cfg.is_empty());
        cfg
    }

    #[test]
    fn test_site_new_good() {
        let cfg = set_default();

        let s = cfg.load("eih");
        assert!(s.is_ok());
    }

    #[test]
    fn test_site_new_unknown() {
        let cfg = set_default();

        let s = cfg.load("bar");
        assert!(s.is_err());
    }

    #[test]
    fn test_site_loading() {
        let s = set_default();

        assert!(!s.is_empty());
        assert_eq!(5, s.len());

        for (name, s) in s.iter() {
            match name.as_str() {
                "eih" => {
                    assert_eq!("aeroscope", s.format);
                    if let Some(auth) = s.auth.clone() {
                        if let Auth::Token { token, login, .. } = auth {
                            assert_eq!("SOMETHING", login);
                            assert_eq!("/login", token);
                        }
                    }
                }
                "asd" => {}
                "opensky" => {}
                "safesky" => {}
                _ => {}
            }
        }
    }

    #[test]
    fn test_site_list() {
        let s = set_default();

        let s = s.get("lux");
        assert!(s.is_some());
        let s = s.unwrap();
        let list = s.list();
        assert_eq!(vec!["get", "list"], list);
    }

    #[test]
    fn test_site_route() {
        let s = set_default();

        let s = s.get("lux");
        assert!(s.is_some());

        let s = s.unwrap();
        let r = s.route("get");
        assert!(r.is_some());

        let r = r.unwrap();
        assert_eq!("/journeys/$1", r);
    }

    #[test]
    fn test_site_has() {
        let s = set_default();

        let s = s.get("lux");
        assert!(s.is_some());

        let s = s.unwrap();
        assert!(s.has("get"));
    }

    #[rstest]
    #[case("adsb", DataType::Adsb)]
    #[case("ads-b", DataType::Invalid)]
    #[case("drone", DataType::Drone)]
    #[case("drones", DataType::Invalid)]
    #[case("foobar", DataType::Invalid)]
    fn test_datatype_from(#[case] s: &str, #[case] dt: DataType) {
        assert_eq!(dt, s.into());
    }
}
