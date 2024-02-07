use chrono::{DateTime, Datelike, TimeZone, Utc};
use clap::Parser;
use duckdb::{params, Connection};
use tracing::info;

#[derive(Debug, Parser)]
pub struct ExportOpts {
    /// Export results for this site
    pub name: String,
    /// Day to export
    pub date: String,
    /// Output file
    #[clap(short = 'o', long)]
    pub output: Option<String>,
}

/// For each considered drone point, export the list of encounters i.e. planes around 1 nm radius
///
#[tracing::instrument(skip(dbh))]
fn export_distances(
    dbh: &Connection,
    name: &str,
    day: DateTime<Utc>,
    fname: &str,
) -> eyre::Result<usize> {
    let r = format!(
        r##"
COPY (
  SELECT * FROM encounters
  WHERE
    site = ? AND
    time >= CAST(? AS DATE) AND
    time < date_add(CAST(? AS DATE), INTERVAL 1 DAY)
    ORDER BY time
) TO '{}' WITH (FORMAT CSV, HEADER true, DELIMITER ',');
        "##,
        fname
    );

    let mut stmt = dbh.prepare(&r)?;
    let count = stmt.execute(params![name, day, day])?;

    Ok(count)
}

#[tracing::instrument(skip(dbh))]
pub fn export_results(dbh: &Connection, opts: ExportOpts) -> eyre::Result<()> {
    let tm = dateparser::parse(&opts.date).unwrap();
    let day = Utc
        .with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0)
        .unwrap();
    info!("Exporting results for {}:", day);

    // Load parameters
    //
    let name = opts.name.clone();

    // Do we export as a csv the "encounters of the day"?
    //
    match &opts.output {
        Some(fname) => {
            let count = export_distances(&dbh, &name, day, fname)?;
            println!("Exported {} records to {}", count, fname);
        }
        None => (),
    }

    info!("Done.");
    Ok(())
}