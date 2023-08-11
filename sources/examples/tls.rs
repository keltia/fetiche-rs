//! Proxy ought to work but torsocks-ify will not.
//!

use std::io::{Read, Write};
use std::net::TcpStream;

use base64::{engine::general_purpose, Engine as _};
use native_tls::TlsConnector;
use reqwest::Url;
use tracing::trace;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

const URL: &str = "www.whatismyip.com";

use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

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

    // Combine filter & specific format
    //
    tracing_subscriber::registry().with(filter).with(fmt).init();

    let opts: Opts = Opts::parse();
    let site = opts.site.unwrap();
    let port = opts.port.unwrap();

    trace!("{}:{}", site, port);

    let proxy = std::env::var("http_proxy").unwrap_or("".to_string());

    let connector = TlsConnector::new()?;
    let stream = if proxy.is_empty() {
        trace!("no proxy");

        TcpStream::connect(format!("{}:{}", site, port))?
    } else {
        trace!("using proxy");

        let url = Url::parse(&proxy)?;
        let (host, port) = (url.host().unwrap(), url.port().unwrap());

        trace!("proxy = {}:{}", host, port);

        let username = url.username();
        let passwd = url.password().unwrap_or("");

        // base64 API is total bullcrap.
        //
        let auth = general_purpose::STANDARD_NO_PAD.encode(format!("{}:{}", username, passwd));

        trace!("CONNECT");
        let mut stream = TcpStream::connect(format!("{}:{}", host, port))?;
        stream.write_all(
            format!(
                "CONNECT {}:{} HTTP/1.1\r\nAuthorization: {}\r\n",
                site, port, auth
            )
            .as_bytes(),
        )?;
        stream
    };
    // Handover to the TLS engine hopefully
    //
    dbg!(&stream);
    let mut stream = connector.connect(&site, stream)?;

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
