//! The `prune` command removes encounter data from the database, like after some
//! metadata changes affecting past calculations, like fixing installation dates.
//!
use crate::{Context, DBVars, make_query, parse_date_interval};
use clap::Parser;
use eyre::Result;
use farben::ceprintln;
use fetiche_common::{DateOpts, jiff_to_chrono};
use klickhouse::{QueryBuilder, RawRow};
use tracing::trace;

#[derive(Debug, Parser)]
pub struct PruneOpts {
    /// Do not execute the command yet.
    #[clap(short = 'f', long)]
    pub force: bool,
    /// Site (or ALL).
    #[clap(short = 's', long, default_value = "ALL")]
    pub site: Option<String>,
    /// Specific date or range to prune.
    #[clap(subcommand)]
    pub date: DateOpts,
}

#[tracing::instrument(skip(ctx))]
pub async fn run_prune_cmd(ctx: &Context, opts: &PruneOpts) -> Result<usize> {
    let dbh = ctx.db().await;
    let profile_name = ctx
        .config
        .get("profile")
        .unwrap_or(&String::from("default"))
        .to_string();
    ceprintln!("[green]Pruning encounters from profile {}.", profile_name);

    let (begin, end) = parse_date_interval(opts.date.clone())?;
    ceprintln!("[green]Pruning encounters between {} and {}", begin, end);

    // Convert jiff Timestamp to chrono only for database formatting
    //
    let date_chrono = jiff_to_chrono(begin)?;
    let time_from = date_chrono.format("%Y-%m-%d 00:00:00").to_string();

    let next_chrono = jiff_to_chrono(end)?;
    let time_to = next_chrono.format("%Y-%m-%d 00:00:00").to_string();
    let dbvars = DBVars::from_ctx(ctx);

    // First count how many
    //
    let r = make_query!(r#"
SELECT count() FROM {workdb}.airplane_prox
WHERE time BETWEEN $1 AND $2
    "#, dbvars);

    let q = QueryBuilder::new(&r).arg(&time_from).arg(&time_to);

    let mut count = match dbh.query_one::<RawRow>(q).await {
        Ok(count) => count,
        Err(_) => {
            trace!("Error counting airplane_prox, assume 0");
            return Ok(0);
        }
    };

    let count: u64 = count.get(0);
    if count == 0 {
        ceprintln!("[green]No encounters to prune between {} and {}", time_from, time_to);
        return Ok(0);
    }

    if !opts.force {
        ceprintln!("[red]Would delete {count} rows, force needed with \"-f\" or \"--force\".");
        return Ok(0);
    }

    // Now, we have told the user we are going to delete these rows, and they have agreed to it.
    //
    let r = make_query!(r#"
DELETE FROM {workdb}.airplane_prox
WHERE time BETWEEN $1 AND $2
    "#, dbvars);

    let q = QueryBuilder::new(&r).arg(&time_from).arg(&time_to);
    let _ = dbh.execute(q).await?;
    trace!("Removing encounters between {} and {}", time_from, time_to);

    Ok(count as usize)
}
