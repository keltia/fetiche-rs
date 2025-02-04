use std::io::Cursor;
use std::ops::Add;
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::{crate_name, crate_version};
use eyre::eyre;
use polars::datatypes::Int64Chunked;
use polars::io::SerWriter;
use polars::prelude::{Column, CsvParseOptions, CsvReadOptions, CsvWriter, IntoColumn, SerReader};
use ractor::cast;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, trace, warn};

use fetiche_formats::Format;

use crate::actors::StatsMsg;
use crate::sources::access::asd::{Credentials, Param, Source, DEF_SOURCES, DEF_TOKEN};
use crate::{Asd, AsdToken, AuthError, Expirable, Fetchable, FetchableSource, Filter, Stats};

impl Fetchable for Asd {
    #[inline]
    fn name(&self) -> String {
        self.site.to_string()
    }

    /// Authenticate to the site using the supplied credentials and get a token
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> eyre::Result<String, AuthError> {
        trace!("authenticate as ({:?})", &self.login);

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
        let token_base = &self.token_base;
        let fname = format!("{}-{}", DEF_TOKEN, self.login);
        let fname = token_base.join(fname);

        let res = if let Ok(token) = AsdToken::retrieve(&fname) {
            // Load potential token data
            //
            trace!("load stored token");
            let token: AsdToken = match serde_json::from_str(&token) {
                Ok(token) => token,
                Err(_) => return Err(AuthError::Invalid(fname.to_string_lossy().to_string())),
            };

            // Check stored token expiration date
            //
            if token.is_expired() {
                // Should we delete it?
                //
                warn!("Stored token in {:?} has expired, deleting!", fname);
                match AsdToken::purge(&fname) {
                    Ok(()) => (),
                    Err(e) => error!("Can not remove token: {}", e.to_string()),
                };
                return Err(AuthError::Expired);
            }
            trace!("token is valid");
            token.token
        } else {
            trace!("no token");

            // fetch token from site
            //
            let url = format!("{}{}", self.base_url, self.token);
            trace!("Fetching token through {}…", url);

            let resp = self
                .client
                .post(url)
                .header("content-type", "application/json")
                .header("user-agent", format!("{}/{}", crate_name!(), crate_version!()))
                .body(json!(&cred).to_string())
                .send()
                .await
                .map_err(|e| AuthError::HTTP(e.to_string()))?;

            trace!("resp={:?}", resp);
            let resp = resp
                .text()
                .await
                .map_err(|_| AuthError::Retrieval(cred.email.clone()))?;

            let res: AsdToken =
                serde_json::from_str(&resp).map_err(|_| AuthError::Decoding(cred.email.clone()))?;

            trace!("token={}", res.token);

            // Write fetched token in `tokens` (unless it is during tests)
            //
            #[cfg(not(test))]
            AsdToken::store(&fname, &resp).map_err(|e| AuthError::Storing(e.to_string()))?;

            res.token
        };

        // Return final token
        //
        Ok(res)
    }

    /// Fetch actual data using the aforementioned token
    ///
    #[tracing::instrument(skip(self))]
    async fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> eyre::Result<()> {
        trace!("asd::fetch");

        const DEF_SOURCES: &[Source] = &[Source::As, Source::Wi];

        let f: Filter = serde_json::from_str(args)?;
        let stats = self.ctx.clone().unwrap().stats;

        // If we have a filter defined, extract times
        //
        let data = match f {
            Filter::Duration(d) => Param {
                start_time: NaiveDateTime::default().and_utc(),
                end_time: NaiveDateTime::default()
                    .and_utc()
                    .add(Duration::try_seconds(d as i64).unwrap()),
                sources: DEF_SOURCES.to_vec(),
            },
            Filter::Interval { begin, end } => Param {
                start_time: begin,
                end_time: end,
                sources: DEF_SOURCES.to_vec(),
            },
            _ => Param {
                start_time: DateTime::<Utc>::MIN_UTC,
                end_time: DateTime::<Utc>::MIN_UTC,
                sources: DEF_SOURCES.to_vec(),
            },
        };

        let data = prepare_asd_data(data);
        debug!("data={}", &data);

        // use token
        //
        let url = format!("{}{}", self.base_url, self.get);
        trace!("Fetching data through {}…", url);

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
            .await?;

        debug!("raw resp={:?}", &resp);

        // Check status
        //
        match resp.status() {
            StatusCode::OK => {}
            code => {
                // This is highly ASD specific
                //
                use percent_encoding::percent_decode;
                trace!("error resp={:?}", resp);
                let h = resp.headers();
                let errtxt = percent_decode(h["x-debug-exception"].as_bytes()).decode_utf8()?;
                let errfile =
                    percent_decode(h["x-debug-exception-file"].as_bytes()).decode_utf8()?;
                return Err(eyre!("Error({}): {} in {}", code, errtxt, errfile));
            }
        }

        // What we receive is an anonymous JSON object containing the filename and CSV content.
        //
        let resp = resp.text().await?;
        trace!("resp={}", resp);
        let data: Payload = serde_json::from_str(&resp)?;

        trace!("Fetched {}", data.filename);

        // We need to fix the timestamp field.
        //
        let cur = Cursor::new(&data.content);
        let opts = CsvParseOptions::default().with_try_parse_dates(false);
        let mut df = CsvReadOptions::default()
            .with_has_header(true)
            .with_parse_options(opts)
            .into_reader_with_file_handle(cur)
            .finish()?;

        // Fix timestamp by replacing the parsed date with its 64-bit timestamp.
        //
        let r = df.apply("timestamp", into_timestamp)?;

        let mut data = vec![];
        CsvWriter::new(&mut data).finish(r)?;

        // Send statistics
        //
        let _ = cast!(stats, StatsMsg::Pkts(data.len() as u32));
        let _ = cast!(stats, StatsMsg::Bytes(resp.len() as u64));

        let data = String::from_utf8(data)?;
        Ok(out.send(data)?)
    }

    /// Return the site's input formats
    ///
    #[inline]
    pub(crate) fn format(&self) -> Format {
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
        .into_iter()
        .map(|d: Option<&str>| d.map(|d: &str| humantime::parse_rfc3339_weak(d).unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64))
        .collect::<Int64Chunked>()
        .into_column()
}

/// ASD is very sensitive to the date format, needs milli-secs.
///
fn prepare_asd_data(data: Param) -> String {
    #[derive(Debug, Serialize)]
    struct P {
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        sources: Vec<&'static str>,
    }

    let p = P { start_time: data.start_time, end_time: data.end_time, sources: DEF_SOURCES.into() };
    json!(&p).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_prepare_asd_data() {
        let start_time = Utc.with_ymd_and_hms(2023, 10, 1, 10, 0, 0).unwrap();
        let end_time = Utc.with_ymd_and_hms(2023, 10, 2, 12, 30, 45).unwrap();
        let data = Param {
            start_time,
            end_time,
            sources: vec![],
        };

        let result = prepare_asd_data(data);

        let expected = json!({
            "start_time": start_time,
            "end_time": end_time,
            "sources": ["as", "wi"]
        })
            .to_string();

        assert_eq!(result, expected);
    }
}
