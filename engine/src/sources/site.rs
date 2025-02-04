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

use fetiche_formats::Format;
use serde::{Deserialize, Serialize};

use crate::{Auth, Capability, Routes};

/// Define the kind of data the source is managing
///
/// This enum represents the type of data handled by a site.
/// It provides a clear differentiation between various types:
///
/// - `Adsb`: Represents plain ADS-B (Automatic Dependent Surveillance–Broadcast) traffic.
/// - `Drone`: Represents drone-specific traffic.
/// - `Invalid`: Serves as a fallback for unrecognized data types.
///
/// The enum supports string-based conversion from common lowercase string representations
/// and implements the `Display` trait for user-friendly formatting.
///
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq, strum::Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum DataType {
    /// Plain ADS-B traffic
    Adsb,
    /// Drone specific traffic
    Drone,
    /// Invalid datatype
    #[default]
    Invalid,
}

/// Represents a `Site` with its configuration details and behavior.
///
/// A `Site` contains metadata about its features, type of data it handles,
/// name, authentication details, and routes. It provides functionalities
/// to interact with the site, such as checking its streamable capability,
/// listing available routes, fetching a specific route, and more.
///
/// ## Fields:
/// - `features`: A list of capabilities that the site supports.
/// - `dtype`: Type of data the site is handling (e.g., ADS-B, Drone).
/// - `name`: The name of the site.
/// - `token_base`: Filesystem path for token storage.
/// - `format`: Data format of the source.
/// - `base_url`: Base URL for accessing the site.
/// - `auth`: Optional authentication details required for the site.
/// - `routes`: Optional list of available routes for the site.
///
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Site {
    /// Features of the site
    pub feature: Capability,
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
    /// Credentials
    pub auth: Option<Auth>,
    /// Different URLs available
    pub routes: Option<Routes>,
}

impl Site {
    /// Basic `new()`
    ///
    #[tracing::instrument]
    #[inline]
    pub fn new() -> Self {
        Site::default()
    }

    /// Return whether a site is streamable
    ///
    #[tracing::instrument]
    #[inline]
    pub fn is_fetchable(&self) -> bool {
        self.feature == Capability::Fetch
    }

    /// Return whether a site is streamable
    ///
    #[tracing::instrument]
    #[inline]
    pub fn is_streamable(&self) -> bool {
        self.feature == Capability::Stream
    }

    /// Return the site name
    ///
    #[tracing::instrument]
    #[inline]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return the site formats
    ///
    #[tracing::instrument]
    #[inline]
    pub fn format(&self) -> Format {
        Format::from_str(&self.format).unwrap()
    }

    /// Return the list of routes
    ///
    #[tracing::instrument]
    #[inline]
    pub fn list(&self) -> Vec<&String> {
        match &self.routes {
            Some(routes) => routes.keys().collect::<Vec<_>>(),
            _ => vec![],
        }
    }

    /// Check whether site has the mentioned route
    ///
    #[tracing::instrument]
    #[inline]
    pub fn has(&self, meth: &str) -> bool {
        match &self.routes {
            Some(routes) => routes.contains_key(meth),
            _ => false,
        }
    }

    /// Retrieve a route
    ///
    #[tracing::instrument]
    #[inline]
    pub fn route(&self, key: &str) -> Option<&String> {
        match &self.routes {
            Some(routes) => routes.get(key),
            _ => None,
        }
    }

    /// Getter for dtype
    ///
    #[tracing::instrument]
    #[inline]
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
    use super::*;
    use crate::Sources;

    use rstest::rstest;

    fn set_default() -> Sources {
        let str = include_str!("sources.hcl");
        let cfg: Sources = hcl::from_str(str).unwrap();
        assert!(!cfg.is_empty());
        cfg
    }

    #[test]
    fn test_site_new_good() {
        let cfg = set_default();

        let s = cfg.get("eih");
        assert!(s.is_some());
    }

    #[test]
    fn test_site_new_unknown() {
        let cfg = set_default();

        let s = cfg.get("bar");
        assert!(s.is_none());
    }

    #[test]
    fn test_site_loading() {
        let s = set_default();

        assert!(!s.is_empty());
        assert_eq!(9, s.len());

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
        assert_eq!(vec!["get", "journey", "journeys"], list);
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
        assert_eq!("/api/journeys/filteredlocations", r);
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
