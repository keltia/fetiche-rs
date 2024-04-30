use std::ops::Add;

use chrono::{Datelike, DateTime, Duration, TimeZone, Utc};
use clickhouse::Client;
use eyre::Result;

const URL: &str = "http://100.92.250.113:8123";
const DB: &str = "acute";
const USER: &str = "default";

const ONE_DEG: f64 = 40_000. / 360.;

#[tracing::instrument(skip(dbh))]
    async fn select_planes(site: u32, date: DateTime<Utc>, lon: f64, lat: f64, dbh: &Client) -> eyre::Result<usize> {
        let day = date.day();
        let month = date.month();
        let year = date.year();

        let distance = 70.;
        // Our distance in nm converted into degrees
        //
        let dist = distance * 1.852 / ONE_DEG;
        println!("{} nm as deg: {}", distance, dist);

        let time_from = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).unwrap();
        let time_to = time_from.add(Duration::try_days(1).unwrap());

        println!("From {} to {}.", time_from, time_to);

        // All flights for a given day in a table
        //
        // $1 = site
        // $2 = year
        // $3 = month
        // $4 = start of day
        // $5 = end of day
        // $6 = lon of site
        // $7 = lat of site
        // $8 = distance in degrees (== dist(nm) /  60)   1 deg ~ 60 nm ~111.1 km
        //
        //
        let r1 = r##"
CREATE OR REPLACE TABLE today
  ORDER BY time
  AS
    SELECT
      site,
      time,
      prox_id AS addr,
      replaceRegexpOne(prox_callsign, '\'([0-9A-Z]+)\\s*\'', '\\1') AS callsign,
      prox_lon AS plon,
      prox_lat AS plat,
      CAST(prox_alt AS DOUBLE) * 0.305 AS palt
    FROM
      airplanes
    WHERE
      site = ? AND
      time BETWEEN timestamp(?) AND timestamp(?) AND
      palt IS NOT NULL AND
      pointInEllipses(plon, plat, ?, ?, ?, ?)
"##;

        // Given lat/lon and dist, we define the "ellipse" aka circle
        // cf. https://clickhouse.com/docs/en/sql-reference/functions/geo/coordinates#pointinellipses
        //
        let x0 = lon;
        let y0 = lat;
        let a0 = y0 + dist;
        let b0 = x0 + dist;
        println!("ellipse=(center={},{},a={},b={})", x0, y0, a0, b0);

        let _ = dbh.query(r1)
            .bind(site)
            .bind(time_from)
            .bind(time_to)
            .bind(x0)
            .bind(y0)
            .bind(a0)
            .bind(b0)
            .execute()
            .await?;

        // Check how many
        //
        let count = dbh.query("SELECT count() FROM today").fetch_one::<usize>().await?;

        println!("Total number of planes: {}\n", count);
        Ok(count)
    }


#[tokio::main]
async fn main() -> Result<()> {
    let name = DB;
    let endpoint = URL;
    let user = USER;
    let date: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 10, 27, 0, 0, 0).unwrap();

    println!("Connecting to {} @ {}", name, endpoint);
    let dbh = Client::default()
            .with_url(endpoint)
            .with_database(name)
            .with_user(user);

    let cnt = select_planes(3, date, 6.2, 49.6, &dbh).await?;
    println!("{cnt} planes found.");
    Ok(())
}