use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use chrono::{DateTime, TimeZone, Utc};
use clap::{crate_name, crate_version};
use eyre::{eyre, Report, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use snafu::Snafu;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::{error, trace, warn};

use fetiche_formats::Format;

use crate::asd::{Asd, Credentials};
use crate::{Capability, HaveToken, http_post, Sources, TOKEN_BASE, TokenError};

/// Default token
const DEF_TOKEN: &str = "asd_default_token";

/// Access token derived from username/password
///
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsdToken {
    /// The actual token
    pub token: String,
    /// Don't ask
    gjrt: String,
    /// Expiration date
    pub expired_at: i64,
    roles: Vec<String>,
    /// Fullname
    name: String,
    supervision: Option<String>,
    lang: String,
    status: String,
    email: String,
    airspace_admin: Option<String>,
    homepage: String,
}

impl Default for AsdToken {
    fn default() -> Self {
        AsdToken {
            token: "".to_owned(),
            gjrt: "".to_owned(),
            expired_at: 0i64,
            roles: vec![],
            name: "John Doe".to_owned(),
            supervision: None,
            lang: "en".to_owned(),
            status: "".to_owned(),
            email: "john.doe@example.net".to_owned(),
            airspace_admin: None,
            homepage: "https://example.net".to_owned(),
        }
    }
}

impl HaveToken for Token {
    fn fetch(&self, name: &str) -> Result<Self> {
        todo!()
    }
}


impl AsdToken {
    /// Retrieve a token from either the storage vault or from the network (if there is no token or
    /// if it has expired).
    ///
}
