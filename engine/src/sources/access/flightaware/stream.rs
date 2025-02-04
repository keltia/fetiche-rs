use std::io::BufReader;

use std::sync::mpsc::Sender;
use tracing::trace;

use crate::{AuthError, Streamable};
use fetiche_formats::Format;

impl Streamable for Flightaware {
    fn name(&self) -> String {
        self.name.to_owned()
    }

    /// All credentials are passed every time we call the API so return a fake token
    ///
    #[tracing::instrument(skip(self))]
    fn authenticate(&self) -> Result<String, AuthError> {
        trace!("fake auth");
        Ok(format!("{}:{}", self.login, self.password))
    }

    /// FIXME: not tested or working
    ///
    fn stream(&self, out: Sender<String>, _token: &str, args: &str) -> Result<()> {
        trace!("stream with TLS");
        let args: Param = serde_json::from_str(args)?;

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
        let mut stream = self.connect(proxy)?;

        // Send request
        //
        trace!("req={req}");
        stream.write_all(req.as_bytes())?;

        trace!("read answer");

        let buf = BufReader::new(&mut stream);
        for line in buf.lines() {
            let line = line.unwrap();
            trace!("line={}", line);
            let _ = out.send(line);
        }

        Ok(())
    }

    fn format(&self) -> Format {
        Format::Flightaware
    }
}
