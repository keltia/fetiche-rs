//! Adapter selection and the shared fetch and stream interfaces.
//!
//! Registry lookups and pipeline producers use the same selectors.

use std::fmt::Debug;
use tokio::sync::mpsc::Sender;

#[cfg(any(
    feature = "asd",
    feature = "aeroscope",
    feature = "safesky",
    feature = "avionix",
    feature = "flightaware",
    feature = "opensky",
    feature = "senhive"
))]
use enum_dispatch::enum_dispatch;
use eyre::Result;
use serde::{Deserialize, Serialize};

use fetiche_formats::Format;

#[cfg(feature = "aeroscope")]
use super::Aeroscope;
#[cfg(feature = "asd")]
use super::Asd;
#[cfg(feature = "flightaware")]
use super::Flightaware;
#[cfg(feature = "safesky")]
use super::Safesky;
#[cfg(feature = "senhive")]
use super::Senhive;
use super::{AccessError, AuthError, Site, Stats};
#[cfg(feature = "avionix")]
use super::{AvionixServer, Cube};
#[cfg(feature = "opensky")]
use super::{OpenskyDevice, OpenskyServer};

#[cfg_attr(
    any(feature = "asd", feature = "aeroscope", feature = "safesky"),
    enum_dispatch
)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum FetchableSource {
    #[cfg(feature = "asd")]
    Asd,
    #[cfg(feature = "aeroscope")]
    Aeroscope,
    #[cfg(feature = "safesky")]
    Safesky,
}

impl FetchableSource {
    /// Resolves an enabled adapter or returns an error for an unavailable site.
    #[tracing::instrument]
    pub fn from_site(value: Site) -> Result<Self> {
        match value.format.to_ascii_lowercase().as_str() {
            #[cfg(feature = "asd")]
            "asd" => {
                let s = Asd::new().load(&value).clone();
                Ok(FetchableSource::from(s))
            }
            #[cfg(feature = "aeroscope")]
            "aeroscope" => {
                let s = Aeroscope::new().load(&value).clone();
                Ok(FetchableSource::from(s))
            }
            #[cfg(feature = "safesky")]
            "safesky" => {
                let s = Safesky::new().load(&value).clone();
                Ok(FetchableSource::from(s))
            }
            _ => Err(AccessError::InvalidSite(value.name).into()),
        }
    }
}

#[cfg_attr(
    any(
        feature = "avionix",
        feature = "flightaware",
        feature = "opensky",
        feature = "senhive"
    ),
    enum_dispatch
)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum StreamableSource {
    #[cfg(feature = "avionix")]
    Cube,
    #[cfg(feature = "avionix")]
    AvionixServer,
    #[cfg(feature = "flightaware")]
    Flightaware,
    #[cfg(feature = "opensky")]
    OpenskyDevice,
    #[cfg(feature = "opensky")]
    OpenskyServer,
    #[cfg(feature = "senhive")]
    Senhive,
}

impl StreamableSource {
    /// Resolves an enabled adapter or returns an error for an unavailable site.
    ///
    /// For `cubedata`, `avionixserver` selects the server and `cube` or
    /// `avionixcube` selects the local device. Variant names are case-insensitive.
    /// Without a variant, user and API keys select the server, while anonymous
    /// or omitted authentication selects the local device. Unknown variants and
    /// incompatible authentication return [`AccessError::InvalidSite`].
    ///
    /// The legacy `opensky` format selects HTTP polling for HTTP endpoints or
    /// login-based authentication, and direct TCP otherwise.
    #[tracing::instrument]
    pub fn from_site(value: Site) -> Result<Self> {
        match value.format.to_ascii_lowercase().as_str() {
            #[cfg(feature = "avionix")]
            "cubedata" => {
                use crate::Auth;

                let variant = value.variant.as_deref().map(str::to_ascii_lowercase);
                match (variant.as_deref(), &value.auth) {
                    (Some("avionixserver"), None | Some(Auth::UserKey { .. }))
                    | (None, Some(Auth::UserKey { .. })) => {
                        Ok(AvionixServer::new().load(&value).clone().into())
                    }
                    (Some("cube" | "avionixcube") | None, None | Some(Auth::Anon)) => {
                        Ok(Cube::new().load(&value).clone().into())
                    }
                    _ => Err(AccessError::InvalidSite(value.name).into()),
                }
            }
            #[cfg(feature = "flightaware")]
            "flightaware" => {
                let s = Flightaware::new().load(&value).clone();
                Ok(StreamableSource::from(s))
            }
            #[cfg(feature = "senhive")]
            "senhive" => {
                let s = Senhive::new().load(&value).clone();
                Ok(StreamableSource::from(s))
            }
            #[cfg(feature = "opensky")]
            format @ ("opensky" | "openskyserver" | "openskydevice") => {
                let server = format == "openskyserver"
                    || (format == "opensky"
                        && (matches!(&value.auth, Some(crate::Auth::Login { .. }))
                            || value.base_url.starts_with("https://")
                            || value.base_url.starts_with("http://")));
                if server {
                    Ok(OpenskyServer::new().load(&value).clone().into())
                } else {
                    Ok(OpenskyDevice::new().load(&value).clone().into())
                }
            }
            _ => Err(AccessError::InvalidSite(value.name).into()),
        }
    }
}

/// This trait enables us to manage different ways of connecting and fetching data under
/// a single interface.
///
#[allow(async_fn_in_trait)]
#[cfg_attr(
    any(feature = "asd", feature = "aeroscope", feature = "safesky"),
    enum_dispatch(FetchableSource)
)]
pub trait Fetchable {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    async fn authenticate(&self) -> Result<String, AuthError>;
    /// Stream actual data
    async fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> Result<Stats>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

/// This trait enables us to manage different ways of connecting and streaming data under
/// a single interface.  The object can connect to a TCP stream or create one by repeatedly calling
/// some API (cf. Opensky).
///
#[allow(async_fn_in_trait)]
#[cfg_attr(
    any(
        feature = "avionix",
        feature = "flightaware",
        feature = "opensky",
        feature = "senhive"
    ),
    enum_dispatch(StreamableSource)
)]
pub trait Streamable: Debug {
    /// Return site's name
    fn name(&self) -> String;
    /// If credentials are needed, get a token for subsequent operations
    async fn authenticate(&self) -> Result<String, AuthError>;
    /// Stream actual data
    async fn stream(&self, out: Sender<String>, token: &str, args: &str) -> Result<Stats>;
    /// Returns the input formats
    fn format(&self) -> Format;
}

// Without adapters these enums are uninhabited. Matching by value makes that
// explicit because the compiler treats references as inhabited.
#[cfg(not(any(feature = "asd", feature = "aeroscope", feature = "safesky")))]
impl Fetchable for FetchableSource {
    /// No fetchable source can be constructed in this build.
    fn name(&self) -> String {
        match *self {}
    }
    /// No fetchable source can authenticate in this build.
    async fn authenticate(&self) -> Result<String, AuthError> {
        match *self {}
    }
    /// No fetchable source can fetch data in this build.
    async fn fetch(&self, _: Sender<String>, _: &str, _: &str) -> Result<Stats> {
        match *self {}
    }
    /// No fetchable source can report a format in this build.
    fn format(&self) -> Format {
        match *self {}
    }
}

#[cfg(not(any(
    feature = "avionix",
    feature = "flightaware",
    feature = "opensky",
    feature = "senhive"
)))]
impl Streamable for StreamableSource {
    /// No streamable source can be constructed in this build.
    fn name(&self) -> String {
        match *self {}
    }
    /// No streamable source can authenticate in this build.
    async fn authenticate(&self) -> Result<String, AuthError> {
        match *self {}
    }
    /// No streamable source can stream data in this build.
    async fn stream(&self, _: Sender<String>, _: &str, _: &str) -> Result<Stats> {
        match *self {}
    }
    /// No streamable source can report a format in this build.
    fn format(&self) -> Format {
        match *self {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sources;

    /// Selects Avionix adapters from explicit variants and legacy authentication.
    #[cfg(feature = "avionix")]
    #[test]
    fn avionix_selection_honors_variant_and_legacy_auth() -> Result<()> {
        let user_key = crate::Auth::UserKey {
            api_key: "test-api-key".into(),
            user_key: "test-user-key".into(),
        };
        for (variant, auth, expected) in [
            (
                Some("AvionixServer"),
                Some(user_key.clone()),
                "AvionixServer",
            ),
            (Some("AvionixServer"), None, "AvionixServer"),
            (Some("avionixcube"), Some(crate::Auth::Anon), "AvionixCube"),
            (Some("cube"), None, "AvionixCube"),
            (None, Some(user_key.clone()), "AvionixServer"),
            (None, Some(crate::Auth::Anon), "AvionixCube"),
            (None, None, "AvionixCube"),
        ] {
            let site = Site {
                name: "avionix".into(),
                format: "CubeData".into(),
                variant: variant.map(str::to_owned),
                auth,
                ..Site::default()
            };
            let sources = Sources::from(vec![(site.name.clone(), site.clone())]);
            assert_eq!(StreamableSource::from_site(site)?.name(), expected);
            assert_eq!(sources.as_streamable("avionix")?.name(), expected);
        }

        for (variant, auth) in [
            (Some("unknown"), None),
            (Some("cube"), Some(user_key)),
            (Some("avionixserver"), Some(crate::Auth::Anon)),
            (
                None,
                Some(crate::Auth::Key {
                    api_key: "test-key".into(),
                }),
            ),
        ] {
            let site = Site {
                name: "avionix".into(),
                format: "cubedata".into(),
                variant: variant.map(str::to_owned),
                auth,
                ..Site::default()
            };
            let err = StreamableSource::from_site(site).unwrap_err();
            assert!(matches!(
                err.downcast_ref(),
                Some(AccessError::InvalidSite(name)) if name == "avionix"
            ));
        }
        Ok(())
    }

    /// Distinguishes a missing registry entry from a known site with an unavailable adapter.
    #[test]
    fn missing_site_returns_unknown_site_for_both_source_kinds() {
        let sources = Sources::from(Vec::<(String, Site)>::new());
        for error in [
            sources.as_fetchable("missing").unwrap_err(),
            sources.as_streamable("missing").unwrap_err(),
        ] {
            assert!(matches!(
                error.downcast_ref(),
                Some(AccessError::UnknownSite(name)) if name == "missing"
            ));
        }
    }

    /// A configured Avionix site is unavailable in both paths when its feature is disabled.
    #[cfg(not(feature = "avionix"))]
    #[test]
    fn disabled_avionix_returns_invalid_site() -> Result<()> {
        let sources: Sources = hcl::from_str(include_str!("sources.hcl"))?;
        let site = sources.get("avionix-rid").unwrap().clone();
        for error in [
            StreamableSource::from_site(site).unwrap_err(),
            sources.as_streamable("avionix-rid").unwrap_err(),
        ] {
            assert!(matches!(
                error.downcast_ref(),
                Some(AccessError::InvalidSite(_))
            ));
        }
        Ok(())
    }

    /// Preserves case-insensitive format matching across the registry and pipeline.
    #[cfg(feature = "asd")]
    #[test]
    fn both_fetch_selectors_accept_mixed_case_formats() -> Result<()> {
        let defaults: Sources = hcl::from_str(include_str!("sources.hcl"))?;
        let mut site = defaults.get("lux").unwrap().clone();
        site.format = "AsD".into();
        let sources = Sources::from(vec![("lux".into(), site.clone())]);
        assert!(matches!(
            FetchableSource::from_site(site)?,
            FetchableSource::Asd(_)
        ));
        assert!(matches!(
            sources.as_fetchable("lux")?,
            FetchableSource::Asd(_)
        ));
        Ok(())
    }

    /// Unavailable adapters return a typed error in every feature configuration.
    #[test]
    fn unavailable_site_returns_error_for_both_source_kinds() {
        let site = Site {
            name: "unavailable".into(),
            format: "unknown-format".into(),
            ..Site::default()
        };
        let sources = Sources::from(vec![(site.name.clone(), site.clone())]);
        for error in [
            FetchableSource::from_site(site.clone()).unwrap_err(),
            StreamableSource::from_site(site).unwrap_err(),
            sources.as_fetchable("unavailable").unwrap_err(),
            sources.as_streamable("unavailable").unwrap_err(),
        ] {
            assert!(matches!(
                error.downcast_ref::<AccessError>(),
                Some(AccessError::InvalidSite(name)) if name == "unavailable"
            ));
        }
    }

    /// The checked-in OpenSky entries select the same adapters through both lookup APIs.
    #[cfg(feature = "opensky")]
    #[test]
    fn configured_opensky_sites_select_the_expected_adapters() -> Result<()> {
        let sources: Sources = hcl::from_str(include_str!("sources.hcl"))?;
        for (name, expected) in [
            ("opensky-server", "openskyserver"),
            ("opensky-local", "openskydevice"),
        ] {
            let site = sources.get(name).unwrap().clone();
            assert_eq!(StreamableSource::from_site(site)?.name(), expected);
            assert_eq!(sources.as_streamable(name)?.name(), expected);
        }
        Ok(())
    }
}
