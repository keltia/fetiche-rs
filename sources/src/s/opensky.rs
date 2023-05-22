//! OpenSky (.org) specific data
//!

use anyhow::anyhow;
use chrono::Utc;
use clap::{crate_name, crate_version};
use log::{debug, trace};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use fetiche_formats::{Cat21, Format};

use crate::{http_get_basic, Fetchable, Filter};
use crate::{Auth, Site};

#[derive(Clone, Debug)]
pub struct Opensky {
    /// Input formats
    pub format: Format,
    /// Username
    pub login: String,
    /// Password
    pub password: String,
    /// Base site url taken from config
    pub base_url: String,
    /// Add this to `base_url` to fetch data
    pub get: String,
    /// reqwest blocking client
    pub client: Client,
}

#[derive(Debug, Serialize)]
struct Param {
    /// timestamp of the state vectors to be retrieved
    pub time: Option<u32>,
    /// One or more ICAO24 transponder address
    pub icao24: Option<Vec<String>>,
    /// One or more receiver IDs
    pub serials: Option<Vec<u32>>,
}

/// Credentials to submit to the site to get the token
///
#[derive(Debug, Serialize)]
struct Credentials {
    /// Email as username
    username: String,
    /// Password
    password: String,
}

impl Opensky {
    pub fn new() -> Self {
        Opensky {
            format: Format::None,
            login: "".to_owned(),
            password: "".to_owned(),
            base_url: "".to_owned(),
            get: "".to_owned(),
            client: Client::new(),
        }
    }

    /// Load some data from the configuration file
    ///
    pub fn load(&mut self, site: &Site) -> &mut Self {
        self.format = site.format.as_str().into();
        self.base_url = site.base_url.to_owned();
        if let Some(auth) = &site.auth {
            match auth {
                Auth::Login {
                    username: login,
                    password,
                } => {
                    self.login = login.to_owned();
                    self.password = password.to_owned();
                }
                _ => panic!("nope"),
            }
        }
        self.get = site.route("get").unwrap().to_owned();
        self
    }
}

impl Default for Opensky {
    fn default() -> Self {
        Self::new()
    }
}

impl Fetchable for Opensky {
    fn authenticate(&self) -> anyhow::Result<String> {
        trace!("fake token retrieval");
        Ok(format!("{}:{}", self.login, self.password))
    }

    fn fetch(&self, token: &str, args: &str) -> anyhow::Result<String> {
        let res: Vec<&str> = token.split(':').collect();
        let (login, password) = (res[0], res[1]);
        trace!("opensky::fetch(as {}:{})", login, password);

        let url = format!("{}{}", self.base_url, self.get);
        trace!("Fetching data from {}…", url);

        let args: Filter = args.into();
        let tm = match args {
            Filter::Interval { begin, .. } => {
                let now = begin.timestamp() as i32;
                Some(format!("time={}", now))
            }
            Filter::Duration(d) => {
                let now = Utc::now().timestamp() as i32;
                Some(format!("time={}", now - d))
            }
            Filter::Keyword { name, value } => Some(format!("{}={}", name, value)),
            Filter::None => None,
        };

        let url = match tm {
            Some(tm) => format!("{}&{}", url, tm),
            _ => url,
        };
        trace!("{}", url);

        let resp = http_get_basic!(self, url, login, password)?;

        debug!("{:?}", &resp);

        // Check status
        //
        match resp.status() {
            StatusCode::OK => {
                trace!("OK");
            }
            code => {
                let h = &resp.headers();
                return Err(anyhow!("Error({}): {:?}", code, h));
            }
        }

        trace!("Fetching raw data");
        let resp = resp.text()?;

        Ok(resp)
    }

    fn to_cat21(&self, _input: String) -> anyhow::Result<Vec<Cat21>> {
        todo!()
    }

    fn format(&self) -> Format {
        Format::Opensky
    }
}

/// Represent the area we want to get all from
///
#[derive(Debug, Serialize, Deserialize)]
struct Args {
    lamin: f32,
    lomin: f32,
    lamax: f32,
    lomax: f32,
}
