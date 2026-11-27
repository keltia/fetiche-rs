//! ASD (Advanced Sensor Data) source data fetching implementation.
//!
//! This module provides functionality to authenticate with and fetch data from ASD data sources.
//! It includes:
//!
//! - Authentication flow using tokens
//! - Data fetching with support for different time intervals
//! - CSV data parsing and timestamp normalization
//! - Error handling for HTTP and data processing operations
//!

use std::io::Cursor;
use std::time::UNIX_EPOCH;
use tokio::sync::mpsc::Sender;

use clap::{crate_name, crate_version};
use eyre::eyre;
use jiff::{Span, Timestamp};
use polars_core::prelude::{Column, Int64Chunked, IntoColumn};
use polars_io::prelude::{CsvParseOptions, CsvReadOptions, CsvWriter, SerReader, SerWriter};
use ractor::{ActorRef, call};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::json;
use tokio::task::spawn_blocking;
use tracing::{info, trace, warn};

use fetiche_formats::Format;

use crate::sources::access::asd::{Credentials, DEF_TOKEN, Param, Source};
use crate::token::AsdToken;
use crate::{
    Asd, AuthError, Expirable, Fetchable, Filter, Stats, TokenMsg, TokenStatus, TokenType,
};

impl Asd {
    /// Uses the engine's shared token store when supplied. Standalone sources use `token_base`.
    #[tracing::instrument(skip(self, tokens))]
    pub(crate) async fn authenticate_with_tokens(
        &self,
        tokens: Option<&ActorRef<TokenMsg>>,
    ) -> eyre::Result<String, AuthError> {
        trace!(source = %self.site, "Authenticating with ASD");

        // Prepare our submission data
        //
        let cred = Credentials {
            email: self.login.clone(),
            password: self.password.clone(),
        };

        // Retrieve token from storage
        //
        // Use `<token>-<email>` to allow identity-based tokens
        //
        let key = format!("{DEF_TOKEN}-{}", self.login);
        let fname = self.token_base.join(&key);
        let tentative = if let Some(tokens) = tokens {
            match call!(tokens, TokenMsg::Get, key.clone())
                .map_err(|err| AuthError::Retrieval(err.to_string()))?
            {
                Ok(TokenType::AsdToken(token)) => token,
                Err(err)
                    if matches!(
                        err.downcast_ref(),
                        Some(TokenStatus::NotFound(_) | TokenStatus::Malformed { .. })
                    ) =>
                {
                    AsdToken::default()
                }
                Err(err) => return Err(AuthError::Retrieval(err.to_string())),
            }
        } else {
            let path = fname.clone();
            spawn_blocking(move || AsdToken::retrieve(&path))
                .await
                .map_err(|err| AuthError::Retrieval(err.to_string()))?
                .map_err(|err| AuthError::Retrieval(err.to_string()))?
        };

        // Now check it.
        //
        info!(source = %self.site, "checking token date");
        let token = if tentative.is_expired() {
            // Either what we retrieved is absent (so we got an invalid token) or is invalid
            //
            warn!(source = %self.site, "Stored token expired or invalid");
            let client = reqwest::Client::new();
            // fetch token from site
            //
            let url = format!("{}{}", self.base_url, self.token);
            trace!(source = %self.site, "Requesting a new token");

            let resp = client
                .post(url)
                .header("content-type", "application/json")
                .header(
                    "user-agent",
                    format!("{}/{}", crate_name!(), crate_version!()),
                )
                .body(json!(&cred).to_string())
                .send()
                .await
                .map_err(|e| AuthError::HTTP(e.without_url().to_string()))?;

            let resp = resp
                .text()
                .await
                .map_err(|_| AuthError::Retrieval("ASD response body".into()))?;

            let res: AsdToken = serde_json::from_str(&resp)
                .map_err(|_| AuthError::Decoding("ASD token response".into()))?;

            if let Some(tokens) = tokens {
                call!(
                    tokens,
                    TokenMsg::Store,
                    key,
                    TokenType::AsdToken(res.clone())
                )
                .map_err(|err| AuthError::Storing(err.to_string()))?
                .map_err(|err| AuthError::Storing(err.to_string()))?;
            } else {
                #[cfg(not(test))]
                spawn_blocking(move || AsdToken::store(&fname, &resp))
                    .await
                    .map_err(|err| AuthError::Storing(err.to_string()))?
                    .map_err(|err| AuthError::Storing(err.to_string()))?;
            }
            res.token
        } else {
            // Valid, not expired, etc.
            //
            tentative.token
        };
        trace!(source = %self.site, "ASD authentication succeeded");
        Ok(token)
    }
}

impl Fetchable for Asd {
    #[inline]
    fn name(&self) -> String {
        self.site.to_string()
    }

    /// Authenticates a standalone ASD source using its configured token directory.
    async fn authenticate(&self) -> eyre::Result<String, AuthError> {
        self.authenticate_with_tokens(None).await
    }

    /// Fetch actual data using the aforementioned token
    ///
    #[tracing::instrument(skip(self, out, token, args))]
    async fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> eyre::Result<Stats> {
        const DEF_SOURCES: &[Source] = &[Source::As, Source::Wi];

        let f: Filter = serde_json::from_str(args)?;

        // If we have a middle defined, extract times
        //
        let data = match f {
            Filter::Duration(d) => {
                let start_time = epoch_timestamp();
                let end_time = start_time
                    .checked_add(Span::new().seconds(d as i64))
                    .unwrap_or(start_time);
                Param {
                    start_time,
                    end_time,
                    sources: DEF_SOURCES.to_vec(),
                }
            }
            Filter::Interval { begin, end } => Param {
                start_time: begin,
                end_time: end,
                sources: DEF_SOURCES.to_vec(),
            },
            _ => Param {
                start_time: epoch_timestamp(),
                end_time: epoch_timestamp(),
                sources: DEF_SOURCES.to_vec(),
            },
        };

        let data = json!(data).to_string();

        // use token
        //
        let url = format!("{}{}", self.base_url, self.get);
        trace!(source = %self.site, "Fetching ASD data");

        let client = reqwest::Client::new();

        // http_post_auth!() macro seems to be disturbing it.
        //
        let resp = client
            .clone()
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", token))
            .body(data)
            .send()
            .await
            .map_err(reqwest::Error::without_url)?;

        let status = resp.status();
        if status != StatusCode::OK {
            return Err(eyre!("ASD request failed with HTTP {status}"));
        }

        // What we receive is an anonymous JSON object containing the filename and CSV content.
        //
        let resp = resp.text().await.map_err(reqwest::Error::without_url)?;
        let bytes = resp.len() as u64;
        let data = spawn_blocking(move || -> eyre::Result<String> {
            let payload: Payload = serde_json::from_str(&resp)?;
            trace!("Fetched {}", payload.filename);
            let opts = CsvParseOptions::default().with_try_parse_dates(false);
            let mut df = CsvReadOptions::default()
                .with_has_header(true)
                .with_parse_options(opts)
                .into_reader_with_file_handle(Cursor::new(payload.content))
                .finish()?;

            let rows = df.apply("timestamp", into_timestamp)?;
            let mut data = Vec::new();
            CsvWriter::new(&mut data).finish(rows)?;
            Ok(String::from_utf8(data)?)
        })
        .await??;

        let stats = Stats {
            tm: Timestamp::now().as_second() as u64,
            pkts: data.len() as u32,
            bytes,
            ..Default::default()
        };

        out.send(data).await?;
        Ok(stats)
    }

    /// Return the site's input formats
    ///
    #[inline]
    fn format(&self) -> Format {
        Format::Asd
    }
}

// -----

/// CSV payload from `.../filteredlocation`
///
#[derive(Debug, Deserialize)]
struct Payload {
    /// Filename if one need to fetch as a file.
    #[serde(rename = "fileName")]
    filename: String,
    /// CSV content is here already.
    content: String,
}

/// Generate a UNIX timestamp from the non-standard date string used by Asd.
///
/// >NOTE: This function is used through polars.
///
fn into_timestamp(col: &Column) -> Column {
    col.str()
        .unwrap()
        .iter()
        .map(|d: Option<&str>| {
            d.map(|d: &str| {
                humantime::parse_rfc3339_weak(d)
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            })
        })
        .collect::<Int64Chunked>()
        .into_column()
}

fn epoch_timestamp() -> Timestamp {
    Timestamp::from_second(0).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Engine authentication uses the actor's cache and standalone authentication uses `token_base`.
    #[tokio::test]
    async fn asd_authenticates_from_the_engine_token_actor() -> eyre::Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().to_path_buf();
        let (actor, handle) =
            ractor::Actor::spawn(None, crate::actors::TokenActor, root.clone()).await?;
        let standalone = tempfile::tempdir()?;
        let source = Asd {
            login: "test@example.invalid".into(),
            token_base: standalone.path().to_path_buf(),
            ..Asd::new()
        };
        let key = format!("{DEF_TOKEN}-{}", source.login);
        let mut token = AsdToken::default();
        token.token = "cached-token".into();
        token.expired_at = crate::time::now_seconds() + 3600;
        call!(
            actor,
            TokenMsg::Store,
            key.clone(),
            TokenType::AsdToken(token.clone())
        )??;
        tokio::fs::remove_file(root.join(&key)).await?;
        token.token = "standalone-token".into();
        tokio::fs::write(source.token_base.join(&key), serde_json::to_vec(&token)?).await?;
        assert_eq!(
            source.authenticate_with_tokens(Some(&actor)).await?,
            "cached-token"
        );
        assert_eq!(source.authenticate().await?, "standalone-token");
        actor.stop(None);
        handle.await?;
        Ok(())
    }

    #[test]
    fn test_clean_asd_data() {
        let start_time: Timestamp = "2023-10-01T10:00:00Z".parse().unwrap();
        let end_time: Timestamp = "2023-10-02T12:30:45Z".parse().unwrap();
        let data = Param {
            start_time,
            end_time,
            sources: vec![Source::As, Source::Wi],
        };

        let result = json!(data).to_string();
        let expected = json!({
            "startTime": start_time,
            "endTime": end_time,
            "sources": ["as", "wi"]
        })
        .to_string();

        assert_eq!(result, expected);
    }
}
