//! This task connects to the database and create some useful macros and tables
//! to set our work environment up.
//!
//! >NOTE: THIS IS CLICKHOUSE-SPECIFIC
//!

use std::env;
use std::path::Path;

mod macros;
mod meta;
mod tables;
mod views;

pub use macros::*;
pub use meta::*;
pub use tables::*;
pub use views::*;

use clap::Parser;
use eyre::Result;
use tracing::trace;

use crate::runtime::Context;

/// Command-line options for setting up the database and environment.
///
/// This structure defines the options for executing the `setup` command,
/// which is responsible for setting up various components of the ACUTE environment.
///
/// ### Options
///
/// - `core`: If enabled (`--core`), creates or removes core raw data tables (airplanes_raw, drones_raw). Requires `--force` flag.
/// - `macros`: If enabled (`-M` or `--macros`), add mathematical macros to the database for distance calculations.
/// - `airplanes`: If enabled (`-P` or `--airplanes`), create the airplanes view for querying airplane data.
/// - `drones`: If enabled (`-D` or `--drones`), create the drones view for querying drone data.
/// - `encounters`: If enabled (`-E` or `--encounters`), create the encounters table to store air-proximity calculation results.
/// - `metadata`: If enabled (`--metadata`), create metadata tables for sites, antennas, and installations.
/// - `records`: If enabled (`-R` or `--records`), create the daily stats table for recording run history.
/// - `views`: If enabled (`-V` or `--views`), create work views for data analysis.
/// - `all`: If enabled (`-a` or `--all`), perform all setup tasks (macros, views, tables, and metadata).
/// - `force`: If enabled (`-F` or `--force`), required safety flag for destructive operations like `--core`.
///
/// ### Examples
///
/// Run the setup command to add database macros:
/// ```sh
/// cargo run -- setup --macros
/// ```
///
/// Create all required tables, views, and macros:
/// ```sh
/// cargo run -- setup --all
/// ```
///
/// Create core tables (requires force flag):
/// ```sh
/// cargo run -- setup --core --force
/// ```
///
/// See also: The `setup_acute_environment` and `cleanup_environment` functions for implementation details.
///
#[derive(Debug, Default, Parser)]
pub struct SetupOpts {
    /// Add / remove core tables.
    #[clap(long)]
    pub core: bool,
    /// Add only macros.
    #[clap(short = 'M', long)]
    pub macros: bool,
    /// Create airplanes views.
    #[clap(short = 'P', long)]
    pub airplanes: bool,
    /// Create drones views.
    #[clap(short = 'D', long)]
    pub drones: bool,
    /// Create encounters (aka calculation) table
    #[clap(short = 'E', long)]
    pub encounters: bool,
    /// Create metadata tables (sites, antennas, installations).
    #[clap(short = 'M', long)]
    pub metadata: bool,
    /// Create records table
    #[clap(short = 'R', long)]
    pub records: bool,
    /// Create work views
    #[clap(short = 'V', long)]
    pub views: bool,
    /// Everything.
    #[clap(short = 'a', long)]
    pub all: bool,
    /// You will need this to use `--core` or `--clean`
    #[clap(short = 'F', long)]
    pub force: bool,
}

// -----

/// Creates or updates components of the ACUTE database environment based on provided options.
///
/// ### Parameters
///
/// - `ctx`: Application context containing database connection and configuration
/// - `opts`: Setup options specifying which components to create/update
///
/// ### Component Setup
///
/// The following components can be created:
/// - Database macros for distance calculations
/// - Views for airplanes data
/// - Views for drones data
/// - Work views for analysis
/// - Encounters table for storing proximity data
/// - Daily stats table for recording run history
///
/// When the `all` flag is set, all components are created. Otherwise, components
/// are created selectively based on individual option flags.
///
/// ### Returns
///
/// Returns `Ok(())` if setup succeeds, or an error if any creation operation fails.
///
/// ### Errors
///
/// May return errors in cases such as:
/// - Database connection issues
/// - Insufficient privileges
/// - SQL syntax errors
/// - Existing components that can't be replaced
///
#[tracing::instrument(skip(ctx))]
pub async fn setup_acute_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    let dir = ctx.config["datalake"].clone();
    let import = Path::new(&dir).join("import");

    // Move here.
    //
    trace!("Moving into {import:?} directory.");
    let _ = env::set_current_dir(&import);

    if opts.all {
        trace!("Creating all ACUTE tables and views.");
        add_macros(&ctx).await?;
        add_sites_table(&ctx).await?;
        add_antennas_table(&ctx).await?;
        add_installations_table(&ctx).await?;
        add_airplanes_view(&ctx).await?;
        add_drones_view(&ctx).await?;
        create_work_views(&ctx).await?;
        let _ = add_encounters_table(&ctx).await;
        let _ = add_daily_stats_table(&ctx).await;
    } else {
        if opts.core {
            trace!("Core tables creation requested.");
            if !opts.force {
                return Err(eyre::eyre!(
                    "You must use --force to create the core tables"
                ));
            }
            create_airplanes_raw_table(ctx).await?;
            create_drones_raw_table(ctx).await?;
        }
        if opts.macros {
            trace!("Creating ACUTE macros.");
            add_macros(&ctx).await?;
        }
        if opts.metadata {
            trace!("Creating ACUTE metadata tables.");
            add_sites_table(&ctx).await?;
            add_antennas_table(&ctx).await?;
            add_installations_table(&ctx).await?;
        }
        if opts.airplanes {
            trace!("Creating ACUTE airplanes table.");
            add_airplanes_view(&ctx).await?;
        }
        if opts.drones {
            trace!("Creating ACUTE drones table.");
            add_drones_view(&ctx).await?;
        }
        if opts.encounters {
            trace!("Creating ACUTE encounters table.");
            add_encounters_table(&ctx).await?;
        }
        if opts.records {
            trace!("Creating ACUTE daily stats table.");
            add_daily_stats_table(&ctx).await?;
        }
        if opts.views {
            trace!("Creating ACUTE views.");
            create_work_views(&ctx).await?;
        }
    }
    Ok(())
}

/// This function removes database components based on the provided options. It can remove
/// all components when the `all` flag is set, or selectively remove specific components
/// based on individual option flags.
///
/// ### Parameters
///
/// - `ctx`: The application context containing database connection and configuration
/// - `opts`: Setup options specifying which components to remove
///
/// ### Component Removal
///
/// The following components can be removed:
/// - Views (work views, deployment views, etc.)
/// - Daily stats table
/// - Encounters table
/// - Drones view
/// - Airplanes view  
/// - Database macros
///
/// ### Returns
///
/// Returns `Ok(())` if cleanup succeeds, or an error if any removal operation fails.
///
/// ### Errors
///
/// May return errors in cases such as:
/// - Database connection issues
/// - Insufficient privileges
/// - Components that don't exist
/// - SQL execution errors
///
#[tracing::instrument(skip(ctx))]
pub async fn cleanup_environment(ctx: &Context, opts: &SetupOpts) -> Result<()> {
    if opts.all {
        drop_work_views(ctx).await?;
        drop_daily_stats_table(ctx).await?;
        drop_encounters_table(ctx).await?;
        drop_drones_view(ctx).await?;
        drop_airplanes_view(ctx).await?;
        drop_sites_table(ctx).await?;
        drop_antennas_table(ctx).await?;
        drop_installations_table(ctx).await?;
        remove_macros(ctx).await?;
    } else {
        if opts.views {
            drop_work_views(&ctx).await?;
        }
        if opts.records {
            drop_daily_stats_table(&ctx).await?;
        }
        if opts.encounters {
            drop_encounters_table(&ctx).await?;
        }
        if opts.drones {
            drop_drones_view(&ctx).await?;
        }
        if opts.airplanes {
            drop_airplanes_view(&ctx).await?;
        }
        if opts.metadata {
            drop_sites_table(&ctx).await?;
            drop_antennas_table(&ctx).await?;
            drop_installations_table(&ctx).await?;
        }
        if opts.macros {
            remove_macros(&ctx).await?;
        }
        if opts.core {
            trace!("Core tables removal requested.");
            if !opts.force {
                return Err(eyre::eyre!(
                    "You must use --force to remove the core tables"
                ));
            }
            drop_drones_raw_table(ctx).await?;
            drop_airplanes_raw_table(ctx).await?;
        }
    }

    Ok(())
}

/// Bootstrapping is a combination of both cleanup/setup to start with a clean slate
///
#[tracing::instrument(skip(ctx))]
pub async fn bootstrap(ctx: &Context) -> Result<()> {
    // Remove everything
    //
    let opts = &SetupOpts {
        all: true,
        ..SetupOpts::default()
    };
    cleanup_environment(ctx, opts).await?;

    // Fiat Lux
    //
    setup_acute_environment(ctx, opts).await?;

    Ok(())
}
