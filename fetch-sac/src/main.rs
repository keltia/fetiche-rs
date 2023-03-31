//! Small CLI utility to fetch the official ASTERIX webpage and scrape the Hell of it in order
//! to get the official list of SAC codes.
//!
//! XXX The fact that I even have to do this is an utter failure on the Agency side.

mod cli;
mod parse;
mod version;

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Parser;
use log::{debug, info};
use regex::Regex;
use reqwest::blocking::get;
use scraper::{Html, Selector};
use stderrlog::LogLevelNum::{Debug, Error, Info, Trace};

use crate::cli::Opts;
use crate::parse::parse_tr;
use crate::version::version;

const PAGE: &str = "https://www.eurocontrol.int/asterix";

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Exit if needed
    //
    if opts.version {
        return Ok(());
    }
    // Check verbosity
    //
    let lvl = match opts.verbose {
        0 => Error,
        1 => Info,
        2 => Debug,
        3 => Trace,
        _ => Trace,
    };

    // Prepare logging.
    stderrlog::new()
        .modules([module_path!()])
        .quiet(opts.quiet)
        .verbosity(lvl)
        .init()?;

    // Add banner
    //
    info!("{}\n", version());

    debug!("Debug mode engaged");

    // Fetch the official page
    //
    let doc = get(PAGE)?.text()?;
    let today: DateTime<Utc> = Utc::now();

    // We want <table> because sometimes there are 3 <td> and sometimes 2 inside a <tr>.
    //
    let sel = Selector::parse("table").unwrap();

    // Parse the page
    //
    let doc = Html::parse_document(&doc);

    // Get all <table>
    //
    let tables = doc.select(&sel).into_iter();

    // Define a regex to sanitize some data
    //
    let re = Regex::new(r##"<br>"##).unwrap();

    // Now look into every table.
    //
    // XXX The 6 tables do not have the same number of cols (aka `<td>`)
    //
    tables.for_each(|e| {
        // For each line
        //
        debug!("frag={:?}", e.html());

        // Now we want each <tr>
        //
        let sel = Selector::parse("tr").unwrap();
        let iter = e.select(&sel).into_iter();

        let res: Vec<_> = iter
            .inspect(|e| debug!("td={e:?}"))
            .map(|e| {
                let frag = e.html().to_owned();

                // Filter
                //
                let frag = re.replace_all(&frag, "");

                // Get what we want
                //
                let (_, (a, b)) = parse_tr(&frag).unwrap();
                format!("num={} tag={}", a, b)
            })
            .collect();

        println!("res={:?}\n", res);
    });
    info!("Information retrieved on: {}", today);
    Ok(())
}
