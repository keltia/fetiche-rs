//! No proxy
//!

use std::io::{Read, Write};
use std::net::TcpStream;

use openssl::ssl::{SslConnector, SslMethod};
use reqwest::Url;
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

    let proxy = std::env::var("http_proxy")?;

    let connector = SslConnector::builder(SslMethod::tls())?.build();
    let stream = if proxy.is_empty() {
        trace!("no proxy");
        TcpStream::connect(format!("{}:{}", URL, PORT))?
    } else {
        trace!("using proxy");

        let url = Url::parse(&proxy)?;
        let (host, port) = (url.host().unwrap(), url.port().unwrap());

        trace!("proxy = {}:{}", host, port);

        let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
        stream.write_all(format!("CONNECT {}:{} HTTP/1.1\r\n\r\n", URL, PORT).as_bytes())?;
        stream
    };
    let mut stream = connector.connect(URL, stream)?;

    let str = format!("GET /\r\nHost: {}\r\nConnection: close\r\n\r\n", URL);

    trace!("read from");
    let mut res = String::new();
    stream.read_to_string(&mut res)?;

    eprintln!("IP={}", res);
    Ok(())
}
