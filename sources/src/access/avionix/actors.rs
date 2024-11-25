use std::fmt::Debug;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::mpsc::Sender;
use std::time::Duration;

use polars::io::SerReader;
use polars::prelude::{JsonFormat, JsonLineReader, JsonReader};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use reqwest::Url;
use tracing::{debug, error, info, trace};

use super::{DEF_PORT, DEF_SITE};
use crate::access::avionix::BUFSIZ;
use crate::actors::StatsMsg;
use crate::Filter;

const START_MARKER: &str = "\x02";

#[derive(Debug)]
pub enum WorkerMsg {
    Consume(Filter, u64),
}

#[derive(Debug)]
pub struct WorkerArgs {
    /// URL to connect to
    pub url: String,
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
        trace!("Starting worker thread");

        let url = Url::parse(state.url.as_str())?;
        let site = url.host_str().unwrap_or(DEF_SITE);
        let port = url.port().unwrap_or(DEF_PORT);

        let user_key = url.password().unwrap_or("");
        let api_key = url.username();

        // Do the connection
        //
        trace!("tcp::connect");
        let mut conn = TcpStream::connect(format!("{site}:{port}")).expect("connect failed");

        let mut conn_in = BufReader::new(&conn);
        let mut conn_out = BufWriter::new(&conn);

        // Send credentials
        //
        let auth_str = format!("{}\n{}\n", api_key, user_key);
        conn_out
            .write_all(auth_str.as_bytes())
            .expect("auth write failed");
        conn_out.flush().expect("flush auth");

        trace!("avionixcube::stream(as {}:{})", api_key, user_key);

        // FIXME: we can have only one argument
        //
        let (stream_duration, params) = match message {
            WorkerMsg::Consume(filter, duration) => {
                let (min, max) = match filter {
                    Filter::Altitude { min, max, .. } => (Some(min), Some(max)),
                    _ => (None, None),
                };
                let duration = Duration::from_secs(duration);
                (duration, (min, max))
            }
        };

        let (min, max) = (params.0, params.1);

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
            let mut buf = [0u8; BUFSIZ];

            match conn_in.read(&mut buf) {
                Ok(size) => {
                    trace!("{} bytes read.", size);
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

                    conn = TcpStream::connect(format!("{site}:{port}")).expect("connect failed");

                    // Do the connection again
                    //
                    conn_in = BufReader::new(&conn);
                    conn_out = BufWriter::new(&conn);
                    continue;
                }
            }
            let data = String::from_utf8(buf.to_vec())?;
            debug!("raw={}", data);

            let cur = Cursor::new(data.as_str());
            let df = JsonReader::new(cur)
                .with_json_format(JsonFormat::JsonLines)
                .infer_schema_len(None).finish()?;

            let _ = stat.cast(StatsMsg::Pkts(df.iter().len() as u32));
            let _ = stat.cast(StatsMsg::Bytes(buf.len() as u64));

            out.send(String::from_utf8(buf.to_vec()).unwrap())
                .expect("send");
        }
    }
}
