use clap::Parser;
use clickhouse::Client;

#[derive(Debug, Parser)]
pub struct Opts {
    pub lat1: f64,
    pub lon1: f64,
    pub lat2: f64,
    pub lon2: f64,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let opts: Opts = Opts::parse();

    let client = Client::default().with_url("http://100.92.250.113:8123");

    let mut res = client.query("SELECT geoDistance(?,?,?,?) AS dist")
        .bind(opts.lon1)
        .bind(opts.lat1)
        .bind(opts.lon2)
        .bind(opts.lat2)
        .fetch::<f64>().unwrap();

    let val: f64 = res.next().await?.unwrap_or(0.);

    println!("Distance between ({},{}) and ({},{})", opts.lat1, opts.lon1, opts.lat2, opts.lon2);
    println!("Distance:\n  {} m clickhouse\n", val);
    Ok(())
}
