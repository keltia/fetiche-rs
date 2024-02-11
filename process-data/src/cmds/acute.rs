use clap::Parser;
use duckdb::arrow::array::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;
use duckdb::Connection;
use eyre::Result;
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

#[tracing::instrument(skip(dbh))]
pub fn run_acute_cmd(dbh: &Connection, opts: AcuteOpts) -> Result<()> {
    trace!("execute");

    match opts.subcmd {
        AcuteSubCommand::Antennas => {
            // Fetch antennas as Arrow
            //
            let mut stmt = dbh.prepare("select * from antennas;")?;

            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            print_batches(&rbs)?;
        }
        AcuteSubCommand::Installations => {
            // Find all installations with sites' name and antenna's ID
            //
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

            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            print_batches(&rbs)?;
        }
        AcuteSubCommand::Sites => {
            // Fetch sites
            //
            let mut stmt = dbh.prepare(
                r##"
SELECT
  name,
  code,
  ST_Point2D(2.35, 48.6) AS ref,
  ST_Point2D(longitude, latitude) AS here,
  deg_to_m(ST_Distance(ref, here)) / 1000 AS distance,
FROM sites
ORDER BY
  name
    "##,
            )?;

            let rbs: Vec<RecordBatch> = stmt.query_arrow([])?.collect();
            print_batches(&rbs)?;
        }
    }

    Ok(())
}
