use std::fs;
use std::io;
use std::str::FromStr;

use clap::{crate_authors, crate_description, crate_version, CommandFactory, Parser};
use clap_complete::generate;
use eyre::{eyre, Result};
use tracing::{info, trace};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;
use tracing_tree::HierarchicalLayer;

use acutectl::{
    convert_from_to, fetch_from_site, stream_from_site, Config, Engine, ImportSubCommand,
    ListSubCommand, Opts, SubCommand,
};
use fetiche_formats::Format;
use fetiche_sources::{Flow, Site};

/// Binary name, using a different binary name
pub const NAME: &str = env!("CARGO_BIN_NAME");
/// Binary version
pub const VERSION: &str = crate_version!();
/// Authors
pub const AUTHORS: &str = crate_authors!();

fn main() -> Result<()> {
    let opts = Opts::parse();
    let cfn = opts.config.clone();

    // Initialise logging early
    //
    let tree = HierarchicalLayer::new(2)
        .with_targets(true)
        .with_higher_precision(true)
        .with_bracketed_fields(true);

    // Setup Open Telemetry with Jaeger
    //
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_auto_split_batch(true)
        .with_max_packet_size(9_216)
        .with_service_name(NAME)
        .install_batch(opentelemetry::runtime::Tokio)?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Load filters from environment
    //
    let filter = EnvFilter::from_default_env();

    // Combine filter & specific format
    //
    tracing_subscriber::registry()
        .with(filter)
        .with(tree)
        .with(telemetry)
        .init();
    trace!("Logging initialised.");

    // Config only has the credentials for every source now.
    //
    let cfg = Config::load(cfn)?;

    // Banner
    //
    banner()?;

    trace!("Engine starting.");
    // Instantiate Engine
    //
    let mut engine = Engine::new();

    // Load auth data
    //
    engine.auth(cfg.site);

    trace!("Engine initialised and running.");

    let subcmd = &opts.subcmd;
    handle_subcmd(&mut engine, subcmd)
}

pub fn handle_subcmd(engine: &mut Engine, subcmd: &SubCommand) -> Result<()> {
    match subcmd {
        // Handle `fetch site`
        //
        SubCommand::Fetch(fopts) => {
            trace!("fetch");

            fetch_from_site(engine, fopts)?;
        }

        // Handle `stream site`
        //
        SubCommand::Stream(sopts) => {
            trace!("stream");

            stream_from_site(engine, sopts)?;
        }

        // Handle `convert from to`
        //
        SubCommand::Convert(copts) => {
            trace!("convert");

            convert_from_to(engine, copts)?;
        }

        // Handle `import site`  and `import file`
        // FIXME:
        //
        SubCommand::Import(opts) => {
            trace!("import");

            match &opts.subcmd {
                ImportSubCommand::ImportSite(fopts) => {
                    trace!("drone import site");

                    let srcs = &engine.sources();
                    let site = match Site::load(&fopts.site, srcs)? {
                        Flow::Fetchable(s) => s,
                        _ => return Err(eyre!("this site is not fetchable")),
                    };
                    let fmt = site.format();

                    // FIXME
                    let data: Vec<u8> = vec![];

                    fetch_from_site(engine, fopts)?;

                    //import_data(&cfg, &data, fmt)?;
                }
                ImportSubCommand::ImportFile(if_opts) => {
                    trace!("drone import file");

                    let data = fs::read_to_string(&if_opts.file)?;
                    let fmt = Format::from_str(&if_opts.format.clone().unwrap())?;

                    //import_data(&srcs, &data, fmt)?;
                }
            }
        }

        // Standalone completion generation
        //
        // NOTE: you can generate UNIX shells completion on Windows and vice-versa.  Not worth
        //       trying to limit depending on the OS.
        //
        SubCommand::Completion(copts) => {
            let generator = copts.shell;
            generate(generator, &mut Opts::command(), NAME, &mut io::stdout());
        }

        // Standalone `list` command
        //
        SubCommand::List(lopts) => match lopts.cmd {
            ListSubCommand::Commands => {
                info!("Listing all commands:");

                let str = engine.list_commands()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Sources => {
                info!("Listing all sources:");

                let str = engine.list_sources()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Formats => {
                info!("Listing all formats:");

                let str = engine.list_formats()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Tokens => {
                info!("Listing all tokens:");

                let str = engine.list_tokens()?;
                eprintln!("{}", str);
            }
            ListSubCommand::Storage => {
                info!("Listing all storage areas:");

                let str = engine.list_storage()?;
                eprintln!("{}", str);
            }
        },

        // Standalone `version` command
        //
        SubCommand::Version => {
            eprintln!("Modules: ");
            eprintln!("\t{}", engine.version());
        }
    }
    Ok(())
}

/// Return our version number
///
#[inline]
pub fn version() -> String {
    format!("{}/{}", NAME, VERSION)
}

/// Display banner
///
fn banner() -> Result<()> {
    Ok(eprintln!(
        r##"
{}/{} by {}
{}
"##,
        NAME,
        VERSION,
        AUTHORS,
        crate_description!()
    ))
}
