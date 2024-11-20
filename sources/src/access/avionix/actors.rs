use crate::access::avionix::BUFSIZ;
use crate::actors::StatsMsg;
use crate::Filter;
use lapin::{Connection, ConnectionProperties};
use polars::prelude::JsonLineReader;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::fmt::Debug;
use std::io::{BufReader, BufWriter, Cursor};
use std::net::Shutdown;
use std::sync::mpsc::Sender;
use tokio::net::TcpStream;
use tracing::{debug, error, info, trace};

#[derive(Debug)]
pub(crate) enum WorkerMsg {
    Consume(String),
}

#[derive(Debug)]
pub(crate) struct WorkerArgs {
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
pub(crate) struct WorkerState {
    /// Connection to the TCP server
    pub conn: TcpStream,
    /// Channel to send data packets to
    pub out: Sender<String>,
    /// Who to send statistics-related events to
    pub stat: ActorRef<StatsMsg>,
}

pub(crate) struct Worker;

/// Worker Actor.
///
/// Do we want one actor for all topics or one actor per topic?
///
#[ractor::async_trait]
impl Actor for Worker {
    type Msg = ();
    type State = WorkerState;
    type Arguments = WorkerArgs;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        trace!("Starting worker actor {}", myself.get_cell().get_id());

        // Do the connection
        //
        trace!("tcp::connect");
        let conn = TcpStream::connect(&args.url).await.expect("connect failed");

        Ok(WorkerState {
            conn,
            out: args.out,
            stat: args.stat,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        trace!("Starting worker thread");

        // Do the connection
        //
        trace!("tcp::connect");
        let mut conn = &state.conn;
        let mut conn_in = BufReader::new(&conn);
        let mut conn_out = BufWriter::new(&conn);

        // Send credentials
        //
        let auth_str = format!("{}\n{}\n", api_key, user_key);
        conn_out
            .write_all(auth_str.as_bytes())
            .expect("auth write failed");

        trace!("avionixcube::stream(as {}:{})", api_key, user_key);

        // FIXME: we can have only one argument
        //
        let (min, max) = match args {
            Filter::Altitude { min, max, .. } => (Some(min), Some(max)),
            _ => (None, None),
        };

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
        let _ = conn_out.write(crate::access::avionix::server::START_MARKER.as_ref());
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
                    stat.send(StatsMsg::Error).expect("stat::error");

                    conn.shutdown(Shutdown::Both).expect("shutdown socket");

                    // We need to drop otherwise `conn`  still remains.
                    //
                    drop(conn_in);
                    drop(conn_out);

                    stat_tx.send(StatsMsg::Reconnect).expect("stat::reconnect");

                    // Do the connection again
                    //
                    conn = std::net::TcpStream::connect(&url).expect("connect socket");
                    conn_in = BufReader::new(&conn);
                    conn_out = BufWriter::new(&conn);
                    continue;
                }
            }
            let cur = Cursor::new(&buf);
            let df = JsonLineReader::new(cur).finish().expect("create dataframe");
            debug!("{:?}", df);

            let _ = stat_tx.send(StatsMsg::Pkts(df.iter().len() as u32));
            let _ = stat_tx.send(StatsMsg::Bytes(buf.len() as u64));

            tx.send(String::from_utf8(buf.to_vec()).unwrap())
                .expect("send");
        }

        todo!()
    }
}
