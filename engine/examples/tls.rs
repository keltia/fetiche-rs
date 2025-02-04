//! Proxy ought to work but torsocks-ify will not.
//!

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;

use base64_light::base64_encode;
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};
use native_tls::TlsConnector;
use reqwest::Url;
use tracing::trace;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

const URL: &str = "www.whatismyipaddress.com";

/// CLI options
#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    #[clap(default_value = URL)]
    pub site: Option<String>,
    #[clap(default_value = "443")]
    pub port: Option<u16>,
}

fn main() -> eyre::Result<()> {
    trace!("open connection");

    let fmt = fmt::layer().with_target(false).compact();

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine middle & specific format
    //
    tracing_subscriber::registry().with(filter).with(fmt).init();

    let opts: Opts = Opts::parse();
    let site = opts.site.unwrap();
    let port = opts.port.unwrap();

    trace!("{}:{}", site, port);

    let proxy = match std::env::var("http_proxy") {
        Ok(s) => Some(s),
        Err(_) => None,
    };

    let connector = TlsConnector::new()?;
    let stream = if proxy.is_none() {
        trace!("no proxy");

        TcpStream::connect(format!("{}:{}", site, port))?
    } else {
        trace!("using proxy");

        let url = Url::parse(&proxy.unwrap())?;
        let (host, port) = (url.host().unwrap(), url.port().unwrap());

        trace!("proxy = {}:{}", host, port);

        let username = url.username();
        let passwd = url.password().unwrap_or("");

        // base64_light API is better.
        //
        let auth = base64_encode(&format!("{}:{}", username, passwd));
        trace!("Auth token is {}", auth);

        trace!("send CONNECT");
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
        stream.write_all(
            format!(
                r##"CONNECT {}:{} HTTP/1.1
Proxy-Authorization: Basic {}
User-Agent: fetiche-rs
Proxy-Connection: Keep-Alive

"##,
                site, 443, auth
            )
                .as_bytes(),
        )?;
        stream
    };

    // Handover to the TLS engine hopefully
    //
    trace!("Over to TLS");
    let mut stream = connector.connect(&site, stream)?;

    dbg!(&stream);

    let str = format!(
        "GET /index.html HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        site
    );
    trace!("{}", str);
    stream.write_all(str.as_bytes())?;

    trace!("READ");

    let out = BufReader::new(stream);
    for data in out.lines() {
        println!("{}", data.unwrap());
    }
    Ok(())
}
