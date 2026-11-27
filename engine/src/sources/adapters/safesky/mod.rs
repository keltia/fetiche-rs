//! Safesky specifics
//!
//! Phase:
//! 1. obtain an API key from a mail sent to them
//! 2. use x-api-key: KEY for all submissions to public-api.safesky.app/beacons
//!
//! Format is take from the CSV given as an example
//!

use eyre::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::trace;

use fetiche_formats::{Format, Position};

use crate::{AccessError, Auth, AuthError, Capability, Fetchable, FetchableSource, Site, Stats};

/// Define the square inside which we want beacons information
///
#[derive(Debug, Deserialize, Serialize)]
struct Viewport {
    /// North-West corner
    nw: Position,
    /// South-East corner
    se: Position,
}

#[allow(dead_code)]
/// Data to submit to get replay of journeys
///
#[derive(Debug, Serialize)]
struct Param {
    /// Limit ourselves to this time interval beginning at
    altitude_min: Option<i32>,
    /// Limit ourselves to this time interval ending at
    altitude_max: Option<i32>,
    /// Beacon types e.g. "UNKNOWN,GLIDER,PARA_GLIDER"
    beacon_type: Option<String>,
    /// Also show grounded beacons?
    show_grounded: bool,
    /// Mandatory:
    viewport: Viewport,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Safesky {
    /// Describe the different features of the source
    pub feature: Capability,
    /// Format of data
    pub format: Format,
    /// Base URL for the API
    pub base_url: String,
    /// Route to get data
    pub get: String,
    /// Safesky uses an API
    pub api_key: String,
}

impl Safesky {
    #[tracing::instrument]
    pub fn new() -> Self {
        trace!("safesky::new");

        Safesky {
            feature: Capability::Fetch,
            format: Format::Safesky,
            base_url: "".to_owned(),
            api_key: "".to_owned(),
            get: "".to_owned(),
        }
    }

    /// Loads the endpoint and API key from a site. Authentication reports a
    /// missing key if the site does not provide `Auth::Key`.
    #[tracing::instrument(skip(self))]
    pub fn load(&mut self, site: &Site) -> &mut Self {
        trace!("safesky::load");

        self.format = Format::Safesky;
        self.base_url = site.base_url.to_owned();
        self.api_key = match &site.auth {
            Some(Auth::Key { api_key }) => api_key.clone(),
            _ => String::new(),
        };
        self.get = site.route("get").cloned().unwrap_or_default();
        self
    }

    #[tracing::instrument(skip(self))]
    pub fn source(&self) -> FetchableSource {
        FetchableSource::Safesky(self.clone())
    }
}

impl Default for Safesky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Safesky {
    fn name(&self) -> String {
        "safesky".to_string()
    }
    /// Safesky is using an API key you need to have for all transactions, there is no
    /// real authentication.
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> Result<String, AuthError> {
        trace!("safesky::authenticate");

        if self.api_key.is_empty() {
            return Err(AuthError::NoAPIKey);
        }
        Ok(self.api_key.clone())
    }

    /// Reports that beacon fetching is unavailable. This path performs no I/O.
    #[tracing::instrument(skip(self, _out, _token, _args))]
    async fn fetch(&self, _out: Sender<String>, _token: &str, _args: &str) -> Result<Stats> {
        Err(AccessError::UnsupportedOperation("SafeSky", "beacon fetching").into())
    }

    fn format(&self) -> Format {
        Format::Safesky
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A configured key authenticates and fetching fails with a typed error.
    #[tokio::test]
    async fn safesky_reports_unavailable_fetching() -> Result<()> {
        let site = Site {
            name: "safe".into(),
            format: "safesky".into(),
            auth: Some(Auth::Key {
                api_key: "test-key".into(),
            }),
            ..Site::default()
        };
        let source = FetchableSource::from_site(site)?;
        let key = source.authenticate().await?;
        assert_eq!(key, "test-key");
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let error = source.fetch(tx, &key, "").await.unwrap_err();
        assert!(matches!(
            error.downcast_ref::<AccessError>(),
            Some(AccessError::UnsupportedOperation(
                "SafeSky",
                "beacon fetching"
            ))
        ));
        assert!(rx.recv().await.is_none());
        Ok(())
    }

    /// Reloading a site without key-based auth clears the previous key.
    #[tokio::test]
    async fn safesky_reload_requires_a_key_from_the_new_site() {
        let mut source = Safesky {
            api_key: "old-key".into(),
            ..Safesky::default()
        };
        source.load(&Site::default());
        assert!(matches!(
            source.authenticate().await,
            Err(AuthError::NoAPIKey)
        ));
    }
}
