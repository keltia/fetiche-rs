use anyhow::{anyhow, Result};
use log::trace;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use format_specs::output::Cat21;
use format_specs::Format;

use crate::aeroscope::Aeroscope;
use crate::asd::Asd;
use crate::config::Sites;
use crate::opensky::Opensky;
use crate::safesky::Safesky;
use crate::Fetchable;

/// Describe what a site is and associated credentials.
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Site {
    /// Type of input
    pub format: String,
    /// Base URL (to avoid repeating)
    pub base_url: String,
    /// Different URLs available
    pub cmd: Routes,
    /// Credentials
    pub auth: Option<Auth>,
}

/// Struct describing the available routes, only `get` to actually fetch data for now
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Routes {
    pub get: String,
}

/// Describe the possible ways to authenticate oneself
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Auth {
    /// Using an API key supplied through the URL or a header
    Key { api_key: String },
    /// Using a login/passwd to get a token
    Token {
        login: String,
        password: String,
        token: String,
    },
    /// Using plain login/password
    Login { username: String, password: String },
    /// Nothing special, no auth
    Anon,
}

impl Site {
    /// Basic `new()`
    ///
    pub fn new() -> Self {
        Site {
            format: "".to_string(),
            base_url: "".to_string(),
            auth: None,
            cmd: Routes {
                get: "".to_string(),
            },
        }
    }

    /// Load site by checking whether it is present in the configuration file
    ///
    pub fn load(name: &str, cfg: &Sites) -> Result<Box<dyn Fetchable>> {
        trace!("Loading site {}", name);
        match cfg.get(name) {
            Some(site) => {
                let fmt = site.format();
                match fmt {
                    Format::Aeroscope => {
                        let s = Aeroscope::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    Format::Asd => {
                        let s = Asd::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    Format::Opensky => {
                        let s = Opensky::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    Format::Safesky => {
                        let s = Safesky::new().load(site).clone();

                        Ok(Box::new(s))
                    }
                    _ => Err(anyhow!("invalid site {name}")),
                }
            }
            None => Err(anyhow!("no such site {name}")),
        }
    }

    /// Return the site format-specs
    ///
    pub fn format(&self) -> Format {
        self.format.as_str().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::config::Sites;
    use crate::makepath;

    fn set_default() -> Sites {
        hcl::from_str(include_str!("config.hcl")).unwrap()
    }

    #[test]
    fn test_site_new_good() {
        let cfg = set_default();

        let s = Site::load("eih", &cfg);
        assert!(s.is_ok());
    }

    #[test]
    fn test_site_new_unknown() {
        let cfg = set_default();

        let s = Site::load("bar", &cfg);
        assert!(s.is_err());
    }

    #[test]
    fn test_site_loading() {
        let s = set_default();

        assert!(!s.is_empty());
        assert_eq!(4, s.len());

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
}
