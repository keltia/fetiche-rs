//! Module to deal with different kind of input be it a file or a site with a, API
//!
//! When no file is specified on the command-line, we look at the list of possible sites to fetch
//! data from, each with a known format.  We also define here the URL and associated credentials
//! (if any) needed.
//!
//! If the `token` URL is present, we call this first with `POST`  to request an OAuth2 token.  
//! We assume the output format to be the same with `{ access_token: String }`.
//!

use crate::format::Source;

use serde::Deserialize;

/// Describe what a site is and associated credentials.
///
#[derive(Debug, Deserialize)]
pub struct Site {
    /// Type of input
    #[serde(rename = "type")]
    pub stype: Source,
    /// Base URL (to avoid repeating)
    pub base_url: String,
    /// Token fetching URL (if present call this first)
    pub token: Option<String>,
    /// Data fetching URL
    pub get: String,
    /// Login (if needed)
    pub login: Option<String>,
    /// Password if needed
    pub password: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Config;
    use std::path::PathBuf;

    #[test]
    fn test_site_loading() {
        let cfn = PathBuf::from("src/config.toml");
        let cfg = Config::load(&cfn);
        assert!(cfg.is_ok());

        let s = &cfg.unwrap();
        assert_eq!("none", s.default);
        assert!(!s.sites.is_empty());
        assert!(3, s.len());
        assert!(s.sites.contains_key("none"));

        assert!(s.sites["none"].login.is_none());
        assert!(Source::None, s.sites["none"].stype);

        assert!(Source::Aeroscope, s.sites["aeroscope"].stype);
        assert!("SOMETHING", s.sites["aeroscope"].login);
    }
}
