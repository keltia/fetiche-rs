//! No proxy
//!

use std::io::{Read, Write};
use std::net::TcpStream;

use base64_light::base64_encode;
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

    let proxy = std::env::var("http_proxy").unwrap_or("".to_string());

    let connector = SslConnector::builder(SslMethod::tls())?.build();
    let stream = if proxy.is_empty() {
        trace!("no proxy");

        TcpStream::connect(format!("{}:{}", URL, PORT))?
    } else {
        trace!("using proxy");

        let url = Url::parse(&proxy)?;
        let (host, port) = (url.host().unwrap(), url.port().unwrap());

        trace!("proxy = {}:{}", host, port);

        let username = url.username();
        let passwd = url.password().unwrap_or("");

        // base64_light API is better.
        //
        let auth = base64_encode(&format!("{}:{}", username, passwd));
        trace!("Auth token is {}", auth);

        trace!("CONNECT");
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
        stream.write_all(
            format!(
                "CONNECT {}:{} HTTP/1.1\r\nAuthorization: {}\r\n",
                URL, PORT, auth
            )
            .as_bytes(),
        )?;
        stream
    };
    // Handover to the TLS engine hopefully
    //
    let mut stream = connector.connect(URL, stream)?;

    trace!("GET");
    let str = format!(
        "GET /index.html\r\nHost: {}\r\nConnection: close\r\n\r\n",
        URL
    );
    stream.write_all(str.as_bytes())?;

    trace!("READ");
    let mut res = String::new();
    stream.read_to_string(&mut res)?;

    eprintln!("IP={}", res);
    Ok(())
}
