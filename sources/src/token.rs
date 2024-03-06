//! Module for handling authentication tokens for some sources.
//!

use std::fmt::Debug;
use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use chrono::{DateTime, TimeZone, Utc};
use clap::{crate_name, crate_version};
use eyre::{eyre, Report, Result};
use reqwest::blocking::Client;
use serde_json::json;
use snafu::Snafu;
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::{trace, warn};
use fetiche_formats::Format;
use crate::{Capability, Sources, TOKEN_BASE};
use crate::asd::Asd;

pub trait HaveToken: Debug {
    fn fetch(&self, name: &str) -> Result<Self>;
    fn store(&self, name: &str) -> Result<()>;
}

/// Custom error type for tokens, allow us to differentiate between errors.
///
#[derive(Debug, PartialEq, Snafu)]
pub enum TokenError {
    #[snafu(display("Can not remove token"))]
    CanNotRemove,
    #[snafu(display("Token expired"))]
    Expired,
    #[snafu(display("Invalid token"))]
    Invalid,
    #[snafu(display("No token."))]
    NoToken,
}

#[derive(Debug)]
pub struct Token<T>
    where T: HaveToken
{
    inner: T,
}

impl<T> Token<T>
    where T: HaveToken
{
    /// Retrieve a token from either the storage vault or from the network (if there is no token or
    /// if it has expired).
    ///
    #[tracing::instrument]
    pub fn retrieve(login: &str) -> Result<Token<T>> {
        // Retrieve token from storage
        //
        // Use `<token>-<email>` to allow identity-based tokens
        //
        let fname = format!("{}-{}", crate::access::asd::token::DEF_TOKEN, login);
        let res = if let Ok(token) = Self::get_token(&fname) {
            // Load potential token data
            //
            trace!("load stored token");
            let token: crate::access::asd::token::AsdToken = match serde_json::from_str(&token) {
                Ok(token) => token,
                Err(_) => return Err(Report::from(TokenError::NoToken)),
            };

            // Check stored token expiration date
            //
            let now: DateTime<Utc> = Utc::now();
            let tok_time: DateTime<Utc> = Utc.timestamp_opt(token.expired_at, 0).unwrap();
            if now > tok_time {
                // Should we delete it?
                //
                warn!("Stored token in {:?} has expired, deleting!", fname);
                match Sources::purge_token(&fname) {
                    Ok(()) => (),
                    Err(e) => return Err(Report::from(TokenError::CanNotRemove)),
                };
                return Err(Report::from(TokenError::Expired));
            }
            trace!("token is valid");
            token
        } else {
            trace!("no token, fetching one");

            let mytok = T::fetch(login)?;
            let token = Token { inner: mytok };
            // Write fetched token in `tokens` (unless it is during tests)
            //
            #[cfg(not(test))]
            token.store(&fname)?;

            token
        };
        Ok(res)
    }

    /// Returns the path of the directory storing tokens
    ///
    pub fn token_path() -> PathBuf {
        Self::config_path().join(TOKEN_BASE)
    }

    /// Return the content of named token
    ///
    #[tracing::instrument]
    pub fn get_token(name: &str) -> Result<String> {
        let t = Self::token_path().join(name);
        trace!("get_token: {t:?}");
        if t.exists() {
            Ok(fs::read_to_string(t)?)
        } else {
            Err(eyre!("{:?}: No such file", t))
        }
    }

    /// Store (overwrite) named token
    ///
    #[tracing::instrument]
    pub fn store(&self) -> Result<()> {
        let p = Self::token_path();

        // Check token cache
        //
        if !p.exists() {
            // Create it
            //
            trace!("create token store: {p:?}");

            fs::create_dir_all(p)?
        }
        let fname = format!("{}-{}", crate::access::asd::token::DEF_TOKEN, self.email);
        let t = Self::token_path().join(fname.into());
        trace!("store_token: {t:?}");

        let data = json!(&self).to_string();
        Ok(fs::write(t, &data)?)
    }

    /// Purge expired token
    ///
    #[tracing::instrument]
    pub fn purge_token(name: &str) -> Result<()> {
        trace!("purge expired token");
        let p = Self::token_path().join(name);
        Ok(fs::remove_file(p)?)
    }

    /// List tokens
    ///
    /// NOTE: we do not show data from each token (like expiration, etc.) because at this point
    ///       we do not know which kind of token each one is.
    ///
    #[tracing::instrument]
    pub fn list_tokens(&self) -> Result<String> {
        trace!("listing tokens");

        let header = vec!["Path", "Created at"];

        let mut builder = Builder::default();
        builder.push_record(header);

        let p = Self::token_path();
        if let Ok(dir) = fs::read_dir(p) {
            for fname in dir {
                let mut row = vec![];

                if let Ok(fname) = fname {
                    // Using strings is easier
                    //
                    let name = format!("{}", fname.file_name().to_string_lossy());
                    row.push(name.clone());

                    let st = fname.metadata().unwrap();
                    let modified = DateTime::<Utc>::from(st.modified().unwrap());
                    let modified = format!("{}", modified);
                    row.push(modified);
                } else {
                    row.push("INVALID".to_string());
                    let origin = format!("{}", DateTime::<Utc>::from(UNIX_EPOCH));
                    row.push(origin);
                }
                builder.push_record(row);
            }
        }
        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all tokens:\n{}", table);
        Ok(table)
    }
}

#[cfg(test)]
mod tests {
    use env_logger;
    use httpmock::prelude::*;
    use serde_json::json;

    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn setup_asd(server: &MockServer) -> Asd {
        init();
        let client = Client::new();
        Asd {
            features: vec![Capability::Fetch],
            site: "NONE".to_string(),
            format: Format::Asd,
            login: "user".to_string(),
            password: "pass".to_string(),
            token: "/api/security/login".to_string(),
            base_url: server.base_url().clone(),
            get: "/api/journeys/filteredlocations/json".to_string(),
            client: client.clone(),
        }
    }

    #[test]
    fn test_get_asd_token() {
        let server = MockServer::start();
        let now = Utc::now().timestamp() + 3600i64;
        let token = crate::access::asd::token::AsdToken {
            token: "FOOBAR".to_string(),
            expired_at: now,
            ..Default::default()
        };

        let jtok = json!(token).to_string();
        let cred = crate::access::asd::Credentials {
            email: "user".to_string(),
            password: "pass".to_string(),
        };
        let cred = json!(cred).to_string();
        let m = server.mock(|when, then| {
            when.method(POST)
                .header(
                    "user-agent",
                    format!("{}/{}", crate_name!(), crate_version!()),
                )
                .header("content-type", "application/json")
                .body(&cred)
                .path("/api/security/login");
            then.status(200).body(&jtok);
        });

        let site = setup_asd(&server);
        let t = site.authenticate();
        dbg!(&t);
        m.assert();
        assert!(t.is_ok());
        assert_eq!("FOOBAR", t.as_ref().unwrap());
    }

    // #[test]
    // fn test_get_asd_fetch() {
    //     let server = MockServer::start();
    //     let filter = Filter::default();
    //     let filter = "{}".to_string();
    //     let token = "FOOBAR".to_string();
    //     let m = server.mock(|when, then| {
    //         when.method(POST)
    //             .header(
    //                 "user-agent",
    //                 format!("{}/{}", crate_name!(), crate_version!()),
    //             )
    //             .header("content-type", "application/json")
    //             .header("authorization", format!("Bearer {}", token))
    //             .path("/api/journeys/filteredlocations/json")
    //             .body(&filter);
    //         then.status(200).body("");
    //     });
    //
    //     let site = setup_asd(&server);
    //     dbg!(&site);
    //
    //     let t = "FOOBAR";
    //     let d = site.fetch(&t, &Filter::default().to_string());
    //
    //     m.assert();
    //     assert!(d.is_ok());
    // }
}
