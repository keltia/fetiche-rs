use clap::Parser;
use eyre::Result;

use crate::cmds::Format;
use crate::config::Context;

#[derive(Debug, Parser)]
pub struct ExpEncOpts {
    /// Export for this day.
    pub date: String,
    /// Output format
    #[clap(short = 'F', long, default_value = "csv")]
    pub format: Format,
    /// Output file
    #[clap(short = 'o', long)]
    pub output: Option<String>,
}

pub fn export_encounters_kml(ctx: &Context, opts: &ExpEncOpts) -> Result<()> {
    /*    let dbh = ctx.db();

        let tm = dateparser::parse(&opts.date).unwrap();
        let day = Utc
            .with_ymd_and_hms(tm.year(), tm.month(), tm.day(), 0, 0, 0)
            .unwrap();
        info!("Exporting results for {}:", day);

        let name = "".to_string();
        let count = 0;
        // Do we export as a csv the "encounters of the day"?
        //
        match &opts.output {
            Some(fname) => {
                let _ = match opts.format {
                    Format::Csv => crate::cmds::export::distances::export_all_encounters_csv(&dbh, &name, day, fname)?,
                    Format::Parquet => crate::cmds::export::distances::export_all_encounters_parquet(&dbh, &name, day, fname)?,
                    _ => 0usize,
                };
                println!("Exported {} records to {}", count, fname);
                0usize
            }
            None => {
                let _ = crate::cmds::export::distances::export_all_encounters_text(&dbh, &name, day)?;
                0usize
            }
        }


    */    Ok(())
}

