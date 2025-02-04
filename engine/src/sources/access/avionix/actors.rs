use std::fmt::Debug;
use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use eyre::Result;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use reqwest::Url;
use serde_json::json;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, trace};

use super::{DEF_PORT, DEF_SITE};
use crate::actors::StatsMsg;
use crate::Filter;
use fetiche_formats::avionix::CubeData;
use fetiche_formats::DronePoint;

const START_MARKER: &str = "\x02";

#[derive(Debug)]
pub enum WorkerMsg {
    Consume(Filter, u64),
}

#[derive(Debug)]
pub struct WorkerArgs {
    /// URL to connect to
    pub url: String,
    /// Filter traffic
    pub traffic: String,
    /// Where to send the data fetched
    pub out: Sender<String>,
    /// For each packet, send statistics data
    pub stat: ActorRef<StatsMsg>,
}

/// Contains the connection handle and the output stream.
/// We also have the address of the stat gathering actor.
///
#[derive(Debug)]
pub struct WorkerState {
    /// Connection to the TCP server
    pub url: String,
    /// Filter traffic
    pub traffic: String,
    /// Channel to send data packets to
    pub out: Sender<String>,
    /// Who to send statistics-related events to
    pub stat: ActorRef<StatsMsg>,
}

pub struct Worker;

/// Worker Actor.
///
/// Do we want one actor for all topics or one actor per topic?
///
#[ractor::async_trait]
impl Actor for Worker {
    type Msg = WorkerMsg;
    type State = WorkerState;
    type Arguments = WorkerArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("Starting worker actor {}", myself.get_cell().get_id());

        Ok(WorkerState {
            url: args.url,
            traffic: String::new(),
            out: args.out,
            stat: args.stat,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            WorkerMsg::Consume(filter, duration) => {
                trace!("Starting worker thread");

                let (min, max) = read_filter(filter);
                let stream_duration = Duration::from_secs(duration);

                let url = Url::parse(state.url.as_str())?;
                let site = url.host_str().unwrap_or(DEF_SITE);
                let port = url.port().unwrap_or(DEF_PORT);

                let user_key = url.password().unwrap_or("");
                let api_key = url.username();

                // Do the connection
                //
                trace!("tcp::connect");
                let mut conn =
                    TcpStream::connect(format!("{site}:{port}")).expect("connect failed");

                let mut conn_in = BufReader::new(&conn);
                let mut conn_out = BufWriter::new(&conn);

                // Send credentials
                //
                let auth_str = format!("{}\n{}\n", api_key, user_key);
                conn_out
                    .write_all(auth_str.as_bytes())
                    .expect("auth write failed");
                conn_out.flush().expect("flush auth");

                trace!("avionix::stream(as {}:{})", api_key, user_key);

                // Manage url parameters.  Assume that if one is defined, the other is as well.
                //
                if min.is_some() {
                    let min = min.unwrap();
                    let min_str = format!("min_altitude={min}\n");
                    let _ = conn_out.write(min_str.as_bytes());
                }
                if max.is_some() {
                    let max = max.unwrap();
                    let max_str = format!("max_altitude={max}\n");
                    let _ = conn_out.write(max_str.as_bytes());
                };

                let out = state.out.clone();

                info!(
                    r##"
StreamURL: {}
Duration {}s
        "##,
                    url,
                    stream_duration.as_secs()
                );

                let stat = state.stat.clone();

                // Start stream
                //
                let _ = conn_out.write(START_MARKER.as_ref());
                conn_out.flush().expect("flush marker");

                trace!("avionixcube::stream started");
                loop {
                    let mut buf = vec![0u8; 4096];

                    let n = match conn_in.read(&mut buf[..]) {
                        Ok(size) => {
                            trace!("{} bytes read.", size);
                            size
                        }
                        Err(e) => {
                            error!("worker-thread: {}", e.to_string());
                            stat.cast(StatsMsg::Error).expect("stat::error");

                            conn.shutdown(Shutdown::Both).expect("shutdown socket");

                            // We need to drop otherwise `conn`  still remains.
                            //
                            drop(conn_in);
                            drop(conn_out);

                            stat.cast(StatsMsg::Reconnect).expect("stat::reconnect");

                            conn = TcpStream::connect(format!("{site}:{port}"))
                                .expect("connect failed");

                            // Do the connection again
                            //
                            conn_in = BufReader::new(&conn);
                            conn_out = BufWriter::new(&conn);

                            // Send credentials again
                            //
                            let auth_str = format!("{}\n{}\n", api_key, user_key);
                            conn_out
                                .write_all(auth_str.as_bytes())
                                .expect("auth write failed");
                            conn_out.flush().expect("flush auth");

                            // Send marker again
                            //
                            let _ = conn_out.write(START_MARKER.as_ref());
                            conn_out.flush().expect("flush marker");

                            continue;
                        }
                    };
                    let raw = String::from_utf8_lossy(&buf[..n]);
                    debug!("raw={}", raw);

                    let filtered = filter_payload(&state.traffic, raw.as_ref())?;

                    let _ = stat.cast(StatsMsg::Pkts(buf.len() as u32));
                    let _ = stat.cast(StatsMsg::Bytes(n as u64));

                    out.send(filtered).expect("send");
                }
            }
        }
    }
}

pub struct LocalWorker;

/// Worker Actor.
///
/// Do we want one actor for all topics or one actor per topic?
///
#[ractor::async_trait]
impl Actor for LocalWorker {
    type Msg = WorkerMsg;
    type State = WorkerState;
    type Arguments = WorkerArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("Starting worker actor {}", myself.get_cell().get_id());

        Ok(WorkerState {
            url: args.url,
            traffic: String::new(),
            out: args.out,
            stat: args.stat,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            WorkerMsg::Consume(filter, duration) => {
                trace!("Starting worker thread");

                let (min, max) = read_filter(filter);
                let stream_duration = Duration::from_secs(duration);

                let url = Url::parse(state.url.as_str())?;
                let site = url.host_str().unwrap_or(DEF_SITE);
                let port = url.port().unwrap_or(DEF_PORT);

                // Do the connection
                //
                trace!("tcp::connect");
                let mut conn =
                    TcpStream::connect(format!("{site}:{port}")).expect("connect failed");

                let mut conn_in = BufReader::new(&conn);
                let mut conn_out = BufWriter::new(&conn);

                trace!("avionix::stream(local)");

                // Manage url parameters.  Assume that if one is defined, the other is as well.
                //
                if min.is_some() {
                    let min = min.unwrap();
                    let min_str = format!("min_altitude={min}\n");
                    let _ = conn_out.write(min_str.as_bytes());
                }
                if max.is_some() {
                    let max = max.unwrap();
                    let max_str = format!("max_altitude={max}\n");
                    let _ = conn_out.write(max_str.as_bytes());
                };

                let out = state.out.clone();

                info!(
                    r##"
        StreamURL: {}
        Duration {}s
                "##,
                    url,
                    stream_duration.as_secs()
                );

                let stat = state.stat.clone();

                // Start stream
                //
                let _ = conn_out.write(START_MARKER.as_ref());
                conn_out.flush().expect("flush marker");

                trace!("avionixcube::stream started");
                loop {
                    let mut buf = vec![0u8; 4096];

                    let n = match conn_in.read(&mut buf[..]) {
                        Ok(size) => {
                            trace!("{} bytes read.", size);
                            size
                        }
                        Err(e) => {
                            error!("worker-thread: {}", e.to_string());
                            stat.cast(StatsMsg::Error).expect("stat::error");

                            conn.shutdown(Shutdown::Both).expect("shutdown socket");

                            // We need to drop otherwise `conn`  still remains.
                            //
                            drop(conn_in);
                            drop(conn_out);

                            stat.cast(StatsMsg::Reconnect).expect("stat::reconnect");

                            conn = TcpStream::connect(format!("{site}:{port}"))
                                .expect("connect failed");

                            // Do the connection again
                            //
                            conn_in = BufReader::new(&conn);
                            conn_out = BufWriter::new(&conn);

                            continue;
                        }
                    };
                    let raw = String::from_utf8_lossy(&buf[..n]);
                    debug!("raw={}", raw);

                    let _ = stat.cast(StatsMsg::Pkts(buf.len() as u32));
                    let _ = stat.cast(StatsMsg::Bytes(n as u64));

                    out.send(String::from_utf8(buf[..n].to_vec())?)
                        .expect("send");
                }
            }
        }
    }
}

// -----

/// Reads the provided `Filter` and extracts the minimum and maximum altitude values.
///
/// This function takes a `Filter` (typically used for configuring altitude ranges
/// or similar constraints) and extracts the `min` and `max` altitude values from it.
/// If the filter does not contain altitude values, it returns `None` for both.
///
/// # Arguments
///
/// * `f` - A `Filter` value that potentially contains altitude range constraints.
///
/// # Returns
///
/// A tuple `(Option<u32>, Option<u32>)` where:
/// - The first element represents the minimum altitude, if present.
/// - The second element represents the maximum altitude, if present.
///
fn read_filter(f: Filter) -> (Option<u32>, Option<u32>) {
    let (min, max) = match f {
        Filter::Altitude { min, max, .. } => (Some(min), Some(max)),
        _ => (None, None),
    };
    (min, max)
}

/// Filters the payload based on the `traffic` type provided.
///
/// This function deserializes the given JSON `payload` into a vector of `CubeData`
/// and filters out the records that do not match the specified `traffic` type.
///
/// The resulting filtered data is then serialized back into a JSON string where
/// each record is separated by a newline (`\n`).
///
/// # Arguments
///
/// * `traffic` - A string slice that specifies the traffic source to filter by. Only records
///   matching this source are retained.
/// * `payload` - A string slice containing the JSON payload to be filtered.
///
/// # Returns
///
/// Returns a `Result` which:
/// * On success, contains the filtered data as a newline-separated JSON string.
/// * On failure, contains a `serde_json::Error` if the payload could not be deserialized.
///
/// # Errors
///
/// This function will return an error if the `payload` is not valid JSON or cannot be
/// deserialized into the expected `CubeData` type.
///
fn filter_payload(traffic: &str, payload: &str) -> Result<String> {
    let data: Vec<CubeData> = serde_json::from_str(payload)?;

    let data: Vec<_> = data
        .iter()
        .filter_map(|line| {
            if line.src == traffic {
                let line = DronePoint::from(line);
                Some(line)
            } else {
                None
            }
        })
        .collect();
    let data = data
        .iter()
        .map(|line| json!(line).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    Ok(data)
}
