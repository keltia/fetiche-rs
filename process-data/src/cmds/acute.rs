use clap::Parser;
use duckdb::arrow::array::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;
use duckdb::Connection;
use eyre::Result;
use tokio::time::Instant;
use tracing::trace;

#[derive(Debug, Parser)]
pub struct AcuteOpts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: String,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    #[clap(subcommand)]
    pub subcmd: AcuteSubCommand,
}

#[derive(Debug, Parser)]
pub enum AcuteSubCommand {
    /// Display all antennas.
    Antennas,
    /// Fetch which antenna was on a site and when.
    Installations,
    /// Display all sites.
    Sites,
}

// -----

pub fn run_acute_cmd(dbh: &Connection, opts: AcuteOpts) -> Result<()> {
    trace!("execute");

    match opts.subcmd {
        AcuteSubCommand::Antennas => {
            // Fetch antennas as Arrow
            //
            let t1 = Instant::now();
            let mut stmt = dbh.prepare("select * from antennas;")?;
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            let t1 = t1.elapsed().as_millis();
            println!("q2 took {}ms", t1);
            print_batches(&rbs)?;
        }
        AcuteSubCommand::Installations => {
            // Find all installations with sites' name and antenna's ID
            //
            let t1 = Instant::now();
            let mut stmt = dbh.prepare(
                r##"
SELECT 
  inst.id,
  sites.name,
  start_at,
  end_at,
  antennas.name AS station_name
FROM
  installations AS inst
  JOIN antennas ON antennas.id = inst.antenna_id
  JOIN sites ON inst.site_id = sites.id
ORDER BY start_at
        "##,
            )?;
            let t1 = t1.elapsed().as_millis();
            println!("prepare took {}ms", t1);

            let t1 = Instant::now();
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            let t1 = t1.elapsed().as_millis();
            println!("q3 took {}ms", t1);
            print_batches(&rbs)?;

            let t1 = Instant::now();
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            let t1 = t1.elapsed().as_millis();
            println!("q4 took {}ms", t1);
            print_batches(&rbs)?;
        }
        AcuteSubCommand::Sites => {
            // Fetch sites
            //
            let t1 = Instant::now();
            let mut stmt = dbh.prepare(
                r##"
SELECT
  name,
  code,
  ST_Point3D(2.35, 48.6,10) AS ref,
  ST_Point3D(longitude, latitude,0) AS here,
  ST_Distance(
    ST_Transform(here, 'EPSG:4326', 'ESRI:102718'), 
    ST_Transform(ref, 'EPSG:4326', 'ESRI:102718') 
  ) / 5280 AS distance
FROM sites
ORDER BY
  name
    "##,
            )?;
            // let res_iter = stmt.query_map([], |row| {
            //     let name: String = row.get_unwrap(0);
            //     let code: String = row.get_unwrap(1);
            //     let coord: Geometry = row.get_unwrap(2);
            //     Ok((name, code, coord))
            // })?;
            // for site in res_iter {
            //     let (n, c, l) = site.unwrap();
            //     println!("site={} code={} coord={:?}", n, c, l);
            // }
            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            let t1 = t1.elapsed().as_millis();
            println!("q1 took {}ms", t1);
            print_batches(&rbs)?;
        }
    }

    Ok(())
}
