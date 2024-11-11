//! Connect to a Thales Senhive antenna and fetch messages through Lapin as AMQP client.
//!

use chrono::{Datelike, Utc};
use eyre::Result;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, Consumer};
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;

use fetiche_common::init_logging;
use fetiche_formats::senhive::{FusedData, StateMsg};

#[derive(Debug, Clone)]
pub enum Output {
    Fixed(String),
    Date(PathBuf),
}

#[derive(Debug)]
pub struct Feed {
    pub name: String,
    pub inp: Consumer,
    pub out: Output,
}

impl Feed {
    pub async fn new(conn: &Connection, name: &str, tag: &str) -> Result<Self> {
        // Create a channel
        let data_ch = conn.create_channel().await?;
        println!("Created {name} channel");

        let data = data_ch
            .basic_consume(
                name,
                tag,
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let now = Utc::now();
        let fname = format!("{:4}{:02}{:02}-{name}", now.year(), now.month(), now.day());
        let fname = Path::new(&fname).with_extension("json");

        Ok(Feed {
            name: name.into(),
            inp: data,
            out: Output::Date(fname),
        })
    }
}

async fn subscribe(conn: &Connection, name: &str) -> Result<Consumer> {
    // Create a channel
    let data_ch = conn.create_channel().await?;
    println!("Created {name} channel");

    let data = data_ch
        .basic_consume(
            name,
            "drone_tag",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;
    Ok(data)
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = env::var("LAPIN_URL").expect("LAPIN_URL must be set");

    // Create logging early.
    //
    init_logging("senhive-amqp", false, false, Some("/acute".to_string()))?;

    // Connect to the AMQP server
    let conn = Connection::connect(&url, ConnectionProperties::default()).await?;
    println!("Connected to RabbitMQ");

    // Subscribe to topics
    //
    let mut data = subscribe(&conn, "fused_data").await?;
    let mut alert = subscribe(&conn, "system_alert").await?;
    let mut state = subscribe(&conn, "system_state").await?;

    let mut dl_data = subscribe(&conn, "dl_fused_data").await?;
    let mut dl_alert = subscribe(&conn, "dl_system_alert").await?;
    let mut dl_state = subscribe(&conn, "dl_system_state").await?;

    println!("Waiting for messages...");

    // setup ctrl-c handled
    //
    #[cfg(windows)]
    let mut sig = ctrl_c().unwrap();

    #[cfg(unix)]
    let mut stream = signal(SignalKind::interrupt()).unwrap();

    let mut fd = fs::File::create("fused_data.json").await?;
    let mut sa = fs::File::create("system_alert.json").await?;
    let mut ss = fs::File::create("system_state.json").await?;

    // Process each message
    //
    loop {
        #[cfg(unix)]
        tokio::select! {
            Some(data) = data.next() => {
                eprint!("D");
                let delivery = data?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: FusedData = serde_json::from_str(&data)?;

                fd.write(&delivery.data).await?;
            },
            Some(data) = dl_data.next() => {
                eprint!("d");
                let delivery = data?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: FusedData = serde_json::from_str(&data)?;

                fd.write(&delivery.data).await?;
            },
            Some(alert) = alert.next() => {
                eprint!("A");
                let delivery = alert?;

                sa.write(&delivery.data).await?;
            },
            Some(alert) = dl_alert.next() => {
                eprint!("a");
                let delivery = alert?;

                sa.write(&delivery.data).await?;
            },
            Some(state) = state.next() => {
                eprint!("S");
                let delivery = state?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: StateMsg = serde_json::from_str(&data)?;

                ss.write(&delivery.data).await?;
            },
            Some(state) = dl_state.next() => {
                eprint!("s");
                let delivery = state?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: StateMsg = serde_json::from_str(&data)?;

                ss.write(&delivery.data).await?;
            },
            Some(_) = stream.recv() => {
                eprintln!("Got SIGINT");
                break;
            },
        }

        #[cfg(windows)]
        tokio::select! {
            Some(data) = data.next() => {
                eprint!("D");
                let delivery = data?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: FusedData = serde_json::from_str(&data)?;

                fd.write(&delivery.data).await?;
            },
            Some(data) = dl_data.next() => {
                eprint!("d");
                let delivery = data?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: FusedData = serde_json::from_str(&data)?;

                fd.write(&delivery.data).await?;
            },
            Some(alert) = alert.next() => {
                eprint!("A");
                let delivery = alert?;

                sa.write(&delivery.data).await?;
            },
            Some(alert) = dl_alert.next() => {
                eprint!("a");
                let delivery = alert?;

                sa.write(&delivery.data).await?;
            },
            Some(state) = state.next() => {
                eprint!("S");
                let delivery = state?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: StateMsg = serde_json::from_str(&data)?;

                ss.write(&delivery.data).await?;
            },
            Some(state) = state.next() => {
                eprint!("s");
                let delivery = state?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                let data_st: StateMsg = serde_json::from_str(&data)?;

                ss.write(&delivery.data).await?;
            },
            _ = sig.recv() => {
                eprintln!("^C pressed.");
                break;
            },
        }
    }
    Ok(())
}
