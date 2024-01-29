use chrono::{DateTime, Utc};
use clap::{crate_authors, crate_description, crate_name, crate_version, Parser};

#[derive(Parser)]
#[command(disable_version_flag = true)]
#[clap(name = crate_name!(), about = crate_description!())]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    /// Database file to use
    #[clap(short = 'd', long)]
    pub database: String,
    /// Output file (default is stdout).
    #[clap(short = 'o', long)]
    pub output: Option<String>,
    /// Sub-commands (see below).
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Debug, Parser)]
pub enum SubCommand {
    /// drone to planes distance
    ToPlanes(PlanesOpts),
    /// List all available modules.
    List,
    /// 2D/3D drone to operator distance.
    ToHome,
    /// Various commands.
    Various,
    /// List all package versions.
    Version,
}

#[derive(Debug, Parser)]
pub struct PlanesOpts {
    /// Do calculation on this date (day).
    pub date: DateTime<Utc>,
    /// Do calculations around this station.
    pub location: String,
    /// Distance in nm
    #[clap(default_value = 70)]
    pub distance: f64,
}
