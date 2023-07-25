use std::fs::File;

use anyhow::Result;
use tracing::trace;

use fetiche_engine::{Convert, Engine, Read};

use crate::ConvertOpts;

#[tracing::instrument]
pub fn convert_from_to(engine: &mut Engine, copts: &ConvertOpts) -> Result<()> {
    trace!("convert_from_to");

    let infile = &copts.infile;
    let outfile = &copts.outfile;
    let from = &copts.from;
    let into = &copts.into;

    // Prepare tasks
    //
    let mut r = Read::new(infile);
    r.path(infile).format(*from);

    let mut c = Convert::new();
    c.from(*from).into(*into);

    // Create job
    //
    let mut j = engine.create_job(&format!("{}->{}", infile, outfile));
    j.add(Box::new(r)).add(Box::new(c));

    let mut fh = File::create(outfile)?;

    j.run(&mut fh)
}
