//! Small CLI utility to fetch the official ASTERIX webpage and scrape the Hell of it in order
//! to get the official list of SAC codes.
//!
//! XXX The fact that I even have to do this is an utter failure on the Agency side.

mod cli;
mod version;

use anyhow::Result;
use clap::Parser;
use log::debug;
use reqwest::blocking::get;
use scraper::{Html, Selector};
use stderrlog::LogLevelNum::{Debug, Info, Trace};

use crate::cli::Opts;
use crate::version::version;

const PAGE: &str = "https://www.eurocontrol.int/asterix";

/// Given a table as string extracted by `scraper`, extract what is relevant
///
fn parse_table(input: &str) -> String {
    debug!("{input}");
    let doc = Html::parse_fragment(input);
    dbg!(&doc);

    // We want <tr> because sometimes there are 3 <td> and sometimes 2.
    //
    let sel = Selector::parse("tr").unwrap();
    let doc = doc.select(&sel).into_iter();

    dbg!(&doc);
    debug!("-----");

    doc.step_by(1)
        .inspect(|e| debug!("{:?}", e.text().collect::<String>()))
        .map(|e| {
            // For each line
            //
            let a1 = e.text().collect::<String>();

            // Get what we want
            //
            let a: Vec<_> = a1.split("\n\t\t").collect();
            let (num, label) = (a[0], a[1]);

            // Sanitise
            //
            let label = label.trim();

            format!("num={} label={}", num, label)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // Add banner
    //
    println!("{}\n", version());

    // Exit if needed
    //
    if opts.version {
        return Ok(());
    }
    // Check verbosity
    //
    let mut lvl = match opts.verbose {
        0 => Info,
        1 => Debug,
        2 => Trace,
        _ => Trace,
    };

    // Prepare logging.
    stderrlog::new()
        .modules(["fetch-sac"])
        .verbosity(lvl)
        .init()?;

    // Fetch the official page
    //
    let doc = get(PAGE)?.text()?;

    // We want <table> because sometimes there are 3 <td> and sometimes 2 inside a <tr>.
    //
    let sel = Selector::parse("table").unwrap();

    // Parse the page
    //
    let doc = Html::parse_document(&doc);

    // Get all <table>
    //
    let tables = doc.select(&sel).into_iter();
    println!("-----");

    // Now look into every table.
    //
    // XXX The 6 tables do not have the same number of cols (aka `<td>`)
    //
    tables.for_each(|e| {
        // For each line
        //
        let frag = Html::parse_document(&e.html());
        debug!("frag={:?}", frag.html());

        // Now we want each <td>
        //
        let sel = Selector::parse("td").unwrap();
        let iter = frag.select(&sel).into_iter();

        let (lower, _upper) = iter.size_hint();
        match lower {
            // 2nd table, US/Canada Region
            2 => iter.map(),
            // All other tables
            3 => {}
            _ => panic!("oopsie"),
        }
        let res: Vec<_> = iter
            .inspect(|e| debug!("td={e:?}"))
            .map(|e| {
                println!("frag_html={}", e.html());
                e.html()
            })
            .collect();

        debug!("res={:?}", res);
    });
    Ok(())
}
