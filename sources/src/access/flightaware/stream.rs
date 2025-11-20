use std::sync::mpsc::Sender;

use ractor::{call, cast};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::trace;

use crate::actors::{StateMsg, StatsMsg};
use crate::{AuthError, Command, Flightaware, Param, Result, Stats, StatsError, Streamable};
use fetiche_formats::Format;

impl Streamable for Flightaware {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    #[tracing::instrument(skip(self))]
    async fn authenticate(&self) -> Result<String, AuthError> {
        trace!("fake auth");
        Ok(format!("{}:{}", self.login, self.password))
    }

    /// FIXME: not tested or working
    ///
    #[tracing::instrument(skip(self, out, _token))]
    async fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<Stats> {
        trace!("stream with TLS");
        let args: Param = serde_json::from_str(args)?;

        let stat = self.stat.clone().ok_or(StatsError::NotInitialized)?;

        let tag = String::from("flightaware::supervisor");
        let _ = cast!(stat, StatsMsg::New(tag.clone()));
        // Check arguments
        //
        let cmd = match args.pitr {
            Some(pitr) => Command::Pitr { pitr },
            None => Command::Live,
        };

        let req = self.request(cmd)?;

        // Setup TLS connection, check proxy environment var first.
        //
        let proxy = match std::env::var("http_proxy") {
            Ok(s) => Some(s),
            Err(_) => None,
        };

        trace!("tls connect");
        let mut stream = self.connect(proxy).await;

        // Send request
        //
        trace!("req={req}");
        let _ = stream.write(req.as_bytes()).await?;

        trace!("read answer");

        let buf = BufReader::new(&mut stream);
        for line in buf.lines().await {
            let line = line.unwrap();
            trace!("line={}", line);
            let _ = out.send(line);
        }

        let stats = call!(stat, |port| StatsMsg::Get(tag.clone(), port))?;
        Ok(stats)
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}
