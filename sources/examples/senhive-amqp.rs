//! Connect to a Thales Senhive antenna and fetch messages through Lapin as AMQP client.
//!

use std::fmt::Debug;
use std::io::Cursor;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::{env, vec};

use chrono::{Datelike, Utc};
use csv::{QuoteStyle, WriterBuilder};
use eyre::Result;
use futures_util::stream::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, Consumer};
use polars::io::{SerReader, SerWriter};
use polars::prelude::{JsonFormat, JsonReader, JsonWriter};
use serde::Serialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
#[cfg(windows)]
use tokio::signal::windows::ctrl_c;
use tracing::trace;

use fetiche_common::init_logging;
use fetiche_formats::senhive::{FusedData, StateMsg};
use fetiche_formats::DronePoint;

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
    let mut data = Feed::new(&conn, "fused_data", "data").await?;
    let mut alert = Feed::new(&conn, "system_alert", "oob").await?;
    let mut state = Feed::new(&conn, "system_state", "state").await?;

    let mut dl_data = Feed::new(&conn, "dl_fused_data", "data").await?;
    let mut dl_alert = Feed::new(&conn, "dl_system_alert", "oob").await?;
    let mut dl_state = Feed::new(&conn, "dl_system_state", "state").await?;

    println!("Waiting for messages...");

    // setup ctrl-c handled
    //
    #[cfg(windows)]
    let mut sig = ctrl_c().unwrap();

    #[cfg(unix)]
    let mut stream = signal(SignalKind::interrupt()).unwrap();

    let mut fd = fs::File::create("fused_data.json").await?;
    let mut fc = fs::File::create("fused_data.csv").await?;
    let mut sa = fs::File::create("system_alert.json").await?;
    let mut ss = fs::File::create("system_state.json").await?;

    // Process each message
    //
    loop {
        #[cfg(unix)]
        tokio::select! {
            Some(data) = data.inp.next() => {
                eprint!("D");
                let delivery = data?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = from_json_to_nl(&delivery.data)?;
                let line = from_json_to_csv(&delivery.data)?;
                fd.write(data.as_bytes()).await?;
                fc.write(line.as_bytes()).await?;

                let _: FusedData = serde_json::from_str(&data)?;
            },
            Some(data) = dl_data.inp.next() => {
                eprint!("d");
                let delivery = data?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = from_json_to_nl(&delivery.data)?;
                let line = from_json_to_csv(&delivery.data)?;
                fd.write(&delivery.data).await?;
                fc.write(line.as_bytes()).await?;

                let _: FusedData = serde_json::from_str(&data)?;
            },
            Some(alert) = alert.inp.next() => {
                eprint!("A");
                let delivery = alert?;

                sa.write(&delivery.data).await?;
            },
            Some(alert) = dl_alert.inp.next() => {
                eprint!("a");
                let delivery = alert?;

                sa.write(&delivery.data).await?;
            },
            Some(state) = state.inp.next() => {
                eprint!("S");
                let delivery = state?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = String::from_utf8_lossy(&delivery.data).to_string();
                ss.write(&delivery.data).await?;

                let _: StateMsg = serde_json::from_str(&data)?;
            },
            Some(state) = dl_state.inp.next() => {
                eprint!("s");
                let delivery = state?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;
                let data = String::from_utf8_lossy(&delivery.data).to_string();
                ss.write(&delivery.data).await?;

                let _: StateMsg = serde_json::from_str(&data)?;
            },
            Some(_) = stream.recv() => {
                eprintln!("Got SIGINT");
                break;
            },
        }

        #[cfg(windows)]
        tokio::select! {
            Some(data) = data.inp.next() => {
                eprint!("D");
                let delivery = data?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = from_json_to_nl(&delivery.data)?;
                let line = from_json_to_csv(&delivery.data)?;
                fd.write(data.as_bytes()).await?;
                fc.write(line.as_bytes()).await?;

                let _: FusedData = serde_json::from_str(&data)?;
            },
            Some(data) = dl_data.inp.next() => {
                eprint!("d");
                let delivery = data?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = from_json_to_nl(&delivery.data)?;
                let line = from_json_to_csv(&delivery.data)?;
                fd.write(data.as_bytes()).await?;
                fc.write(line.as_bytes()).await?;

                let _: FusedData = serde_json::from_str(&data)?;
            },
            Some(alert) = alert.inp.next() => {
                eprint!("A");
                let delivery = alert?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                sa.write(&delivery.data).await?;
            },
            Some(alert) = dl_alert.inp.next() => {
                eprint!("a");
                let delivery = alert?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                sa.write(&delivery.data).await?;
            },
            Some(state) = state.inp.next() => {
                eprint!("S");
                let delivery = state?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = from_json_to_nl(&delivery.data)?;
                ss.write(data.as_bytes()).await?;

                let _: StateMsg = serde_json::from_str(&data)?;
            },
            Some(state) = dl_state.inp.next() => {
                eprint!("s");
                let delivery = state?;
                delivery
                    .ack(BasicAckOptions::default())
                    .await?;

                let data = from_json_to_nl(&delivery.data)?;
                ss.write(data.as_bytes()).await?;

                let _: StateMsg = serde_json::from_str(&data)?;
            },
            _ = sig.recv() => {
                eprintln!("^C pressed.");
                break;
            },
        }
    }
    Ok(())
}

fn from_json_to_nl(data: &[u8]) -> Result<String> {
    let cur = Cursor::new(data);
    let mut df = JsonReader::new(cur)
        .with_json_format(JsonFormat::Json)
        .infer_schema_len(NonZeroUsize::new(3))
        .finish()?;
    let mut buf = vec![];
    //let mut res = BufWriter::new(&mut buf);
    JsonWriter::new(&mut buf)
        .with_json_format(JsonFormat::JsonLines)
        .finish(&mut df)?;
    Ok(String::from_utf8(buf)?)
}

fn from_json_to_csv(data: &[u8]) -> Result<String> {
    let cur = Cursor::new(data);
    let data: FusedData = serde_json::from_reader(cur)?;
    let data: DronePoint = data.into();

    let data = prepare_csv(data)?;
    Ok(data)
}

#[tracing::instrument]
pub fn prepare_csv<T>(data: T) -> Result<String>
where
    T: Serialize + Debug,
{
    trace!("Generating output…");
    // Prepare the writer
    //
    let mut wtr = WriterBuilder::new()
        .has_headers(false)
        .quote_style(QuoteStyle::NonNumeric)
        .from_writer(vec![]);

    // Insert data
    //
    wtr.serialize(data)?;

    // Output final csv
    //
    let data = String::from_utf8(wtr.into_inner()?)?;
    Ok(data)
}
