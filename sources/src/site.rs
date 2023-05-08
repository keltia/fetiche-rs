use std::fmt::{Debug, Display, Formatter};

use anyhow::{anyhow, Result};
use log::trace;
use serde::{Deserialize, Serialize};

use format_specs::Format;

use crate::config::Sites;
use crate::Fetchable;
use crate::{aeroscope::Aeroscope, asd::Asd, opensky::Opensky, safesky::Safesky};

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

#[macro_export]
macro_rules! insert_format {
    ($name:ident, $fmt:ident, $site:ident, $($list:ident),+)  => {
        match $fmt {
        $(
            Format::$list => {
                let s = $list::new().load($site).clone();
                Ok(Box::new(s))
            },
        )+
            _ => Err(anyhow!("invalid site {}", $name)),
        }
    }
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
                insert_format!(name, fmt, site, Asd, Aeroscope, Safesky, Opensky)
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

impl Default for Site {
    fn default() -> Self {
        Site::new()
    }
}

impl Display for Site {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Hide passwords & API keys
        //
        let mut site = self.clone();
        if let Some(auth) = site.auth {
            let auth = match auth {
                Auth::Key { .. } => Auth::Key {
                    api_key: "HIDDEN".to_string(),
                },
                Auth::Login { username, .. } => Auth::Login {
                    username,
                    password: "HIDDEN".to_string(),
                },
                Auth::Token { login, token, .. } => Auth::Token {
                    login,
                    token,
                    password: "HIDDEN".to_string(),
                },
                _ => Auth::Anon,
            };
            site.auth = Some(auth.clone());
        }
        write!(f, "{:?}", site)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::config::Sites;
    use crate::makepath;

    use super::*;

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
