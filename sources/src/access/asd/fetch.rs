use std::io::Cursor;
use std::ops::Add;
use std::sync::mpsc::Sender;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use clap::{crate_name, crate_version};
use eyre::eyre;
use polars::io::SerWriter;
use polars::prelude::{CsvParseOptions, CsvReadOptions, CsvWriter, SerReader};
use ractor::cast;
use reqwest::StatusCode;
use tracing::{debug, error, trace, warn};

use fetiche_formats::Format;

use crate::access::asd::{
    into_timestamp, prepare_asd_data, Credentials, Param, Payload, Source, DEF_TOKEN,
};
use crate::actors::StatsMsg;
use crate::{http_post, Asd, AsdToken, AuthError, Expirable, Fetchable, Filter};

impl Fetchable for Asd {
    fn name(&self) -> String {
        self.site.to_string()
    }

    /// Authenticate to the site using the supplied credentials and get a token
    ///
    #[tracing::instrument(skip(self))]
    fn authenticate(&self) -> eyre::Result<String, AuthError> {
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

        let res = if let Ok(token) = Asd::retrieve(&fname) {
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
                match Asd::purge(&fname) {
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
            let resp = http_post!(self, url, &cred).map_err(|e| AuthError::HTTP(e.to_string()))?;

            trace!("resp={:?}", resp);
            let resp = resp
                .text()
                .map_err(|_| AuthError::Retrieval(cred.email.clone()))?;

            let res: AsdToken =
                serde_json::from_str(&resp).map_err(|_| AuthError::Decoding(cred.email.clone()))?;

            trace!("token={}", res.token);

            // Write fetched token in `tokens` (unless it is during tests)
            //
            #[cfg(not(test))]
            Asd::store(&fname, &resp).map_err(|e| AuthError::Storing(e.to_string()))?;

            res.token
        };

        // Return final token
        //
        Ok(res)
    }

    /// Fetch actual data using the aforementioned token
    ///
    #[tracing::instrument(skip(self))]
    fn fetch(&self, out: Sender<String>, token: &str, args: &str) -> eyre::Result<()> {
        trace!("asd::fetch");

        const DEF_SOURCES: &[Source] = &[Source::As, Source::Wi];

        let f: Filter = serde_json::from_str(args)?;

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

        // http_post_auth!() macro seems to be disturbing it.
        //
        let resp = self
            .client
            .clone()
            .post(url)
            .header(
                "user-agent",
                format!("{}/{}", crate_name!(), crate_version!()),
            )
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", token))
            .body(data)
            .send()?;

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
        let resp = resp.text()?;
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
        let _ = cast!(self.ctx.stats, StatsMsg::Pkts(data.len() as u32));
        let _ = cast!(self.ctx.stats, StatsMsg::Bytes(resp.len() as u64));

        let data = String::from_utf8(data)?;
        Ok(out.send(data)?)
    }

    /// Return the site's input formats
    ///
    fn format(&self) -> Format {
        Format::Asd
    }
}
