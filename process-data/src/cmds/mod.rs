//! Commands module for processing data.
//!
//! This module contains implementations of various commands for data processing:
//! - Distance calculations between drones and planes
//! - Data export functionality for distances, drones and encounters
//! - Environment setup and cleanup
//! - Status checking and bootstrapping
//!
//! The module provides trait `Calculate` that defines calculation interface
//! and function `handle_cmds` for routing and executing command requests.
//!

use std::fmt::Debug;

use fetiche_common::Delim;
use klickhouse::Client;
use serde::Serialize;
use serde_json::json;
use tabled::settings::Style;
use tabled::{Table, Tabled};
use tracing::info;

pub use acute::*;
pub use check::*;
pub use dates::*;
pub use distances::*;
pub use error::*;
pub use export::*;
pub use profile::*;
pub use prune::*;
pub use query::*;
//pub use record::*;
pub use setup::*;
pub use site::*;
pub use stats::*;

use crate::cli::{Opts, SubCommand};
use crate::runtime::Context;

mod acute;
mod check;
mod dates;
mod distances;
mod error;
mod export;
mod profile;
mod prune;
mod query;
//mod record;
mod setup;
mod site;
mod stats;

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;

#[allow(async_fn_in_trait)]
/// This trait defines an object that can be calculated
///
pub trait Calculate: Debug {
    async fn run(&mut self, dbh: &Client) -> eyre::Result<Stats>;
}

/// Generic helper to render serializable and tabular data into the requested output format.
pub fn render_output<T: Debug + Serialize + Tabled>(
    data: &[T],
    format: Format,
) -> eyre::Result<String> {
    match format {
        Format::Json => Ok(json!(data).to_string()),
        Format::Csv => Delim::Comma.prepare_csv(data, true),
        Format::Table | Format::Text => {
            let mut table = Table::new(data);
            table.with(Style::sharp());
            Ok(table.to_string())
        }
        Format::Ndjson => {
            let mut out = String::new();
            for item in data {
                out.push_str(&serde_json::to_string(item)?);
                out.push('\n');
            }
            Ok(out)
        }
        _ => eyre::bail!("Unsupported format for text rendering: {}", format),
    }
}

// -----

#[tracing::instrument(skip(ctx))]
pub async fn handle_cmds(ctx: &Context, opts: &Opts) -> eyre::Result<()> {
    match &opts.subcmd {
        SubCommand::Distances(dopts) => match &dopts.subcmd {
            DistSubcommand::Planes(popts) => {
                eprintln!("Calculate 3D distance between drones and surrounding planes.\n");

                let stats = planes_calculation(ctx, popts).await?;
                eprintln!("Stats:\n{:?}", stats);
            }
        },
        SubCommand::Export(eopts) => match &eopts.subcmd {
            ExportSubCommand::Distances(opts) => {
                eprintln!("Exporting calculated distances.\n");

                export_results(ctx, opts).await?;
            }
            ExportSubCommand::Drones(opts) => {
                eprintln!("Exporting drone data.\n");

                export_drone_stats(ctx, opts).await?;
            }
            ExportSubCommand::Encounters(opts) => {
                eprintln!("Exporting encounters.\n");

                export_encounters(ctx, opts).await?;
            }
        },
        SubCommand::Check(_copts) => {
            eprintln!("Check daily runs status.");
            todo!()
        }
        SubCommand::Setup(sopts) => {
            eprintln!("Setup ACUTE environment in {}.\n", ctx.config["datalake"]);
            setup_acute_environment(ctx, sopts).await?;
        }
        SubCommand::Cleanup(copts) => {
            eprintln!("Remove ACUTE environement in {}.\n", ctx.config["datalake"]);
            cleanup_environment(ctx, copts).await?;
        }
        SubCommand::Profile(ptopts) => {
            run_profile_cmd(ctx, ptopts)?;
        }
        SubCommand::Prune(popts) => {
            eprintln!("Purging some results from encounters.\n");
            let count = run_prune_cmd(ctx, popts).await?;
            eprintln!("Removed {} rows.", count);
        }
        SubCommand::Acute(aopts) => {
            eprintln!("ACUTE specific commands.\n");
            run_acute_cmd(ctx, aopts).await?;
        }
        SubCommand::Bootstrap => {
            eprintln!("Initial bootstrapping.");
            bootstrap(ctx).await?;
        }
        // These are done already.
        //
        _ => (),
    }

    info!("Closing DB.");
    let _ = ctx.finish();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Tabled)]
    struct DummyItem {
        id: u32,
        name: String,
    }

    #[test]
    fn test_render_output_json() {
        let items = vec![
            DummyItem {
                id: 1,
                name: "Foo".into(),
            },
            DummyItem {
                id: 2,
                name: "Bar".into(),
            },
        ];
        let out = render_output(&items, Format::Json).unwrap();
        assert!(out.contains(r#""id":1"#));
        assert!(out.contains(r#""name":"Foo""#));
    }

    #[test]
    fn test_render_output_csv() {
        let items = vec![
            DummyItem {
                id: 1,
                name: "Foo".into(),
            },
            DummyItem {
                id: 2,
                name: "Bar".into(),
            },
        ];
        let out = render_output(&items, Format::Csv).unwrap();
        assert!(out.contains("id,name"));
        assert!(out.contains("1,Foo"));
        assert!(out.contains("2,Bar"));
    }

    #[test]
    fn test_render_output_table() {
        let items = vec![DummyItem {
            id: 42,
            name: "TableTest".into(),
        }];
        let out = render_output(&items, Format::Table).unwrap();
        assert!(out.contains("42"));
        assert!(out.contains("TableTest"));
    }

    #[test]
    fn test_render_output_ndjson() {
        let items = vec![
            DummyItem {
                id: 1,
                name: "Foo".into(),
            },
            DummyItem {
                id: 2,
                name: "Bar".into(),
            },
        ];
        let out = render_output(&items, Format::Ndjson).unwrap();
        let lines: Vec<&str> = out.trim().lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains(r#""id":1"#));
        assert!(lines[1].contains(r#""id":2"#));
    }

    #[test]
    fn test_render_output_unsupported() {
        let items = vec![DummyItem {
            id: 1,
            name: "Test".into(),
        }];
        let err = render_output(&items, Format::Parquet);
        assert!(err.is_err());
    }
}
