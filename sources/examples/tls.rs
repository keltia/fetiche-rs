//! No proxy
//!

use std::io::{Read, Write};
use std::net::TcpStream;

use native_tls::TlsConnector;
use tracing::trace;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

const URL: &str = "www.whatismyip.com";
const PORT: u16 = 443;

fn main() -> eyre::Result<()> {
    trace!("open connection");

    let fmt = fmt::layer().with_target(false).compact();

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry().with(filter).with(fmt).init();

    //let proxy = "proxysrv.eurocontrol.fr:8080";

    let connector = TlsConnector::new()?;
    let stream = TcpStream::connect(format!("{}:{}", URL, PORT))?;
    let mut stream = connector.connect(URL, stream)?;

    let str = format!("GET /\r\nHost: {}\r\nConnection: close\r\n\r\n", URL,);
    stream.write_all(str.as_bytes())?;

    trace!("read from");
    let mut res = String::new();
    stream.read_to_string(&mut res)?;

    eprintln!("IP={}", res);
    Ok(())
}
