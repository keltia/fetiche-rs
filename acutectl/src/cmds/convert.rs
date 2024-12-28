use std::fs::File;

use eyre::Result;
use tracing::trace;

use fetiche_engine::{Convert, Engine, Read, Task};

use crate::ConvertOpts;

#[tracing::instrument]
pub async fn convert_from_to(engine: &mut Engine, copts: &ConvertOpts) -> Result<()> {
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

    let t1 = Task::from(r);
    let t2 = Task::from(c);

    // Create job
    //
    let mut j = engine
        .create_job(&format!("{}->{}", infile, outfile))
        .await?;
    j.add(t1).add(t2);

    let mut fh = File::create(outfile)?;

    j.run(&mut fh)
}
