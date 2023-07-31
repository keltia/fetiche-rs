use std::collections::BTreeMap;

use eyre::Result;
use tracing::{info, trace};

use fetiche_engine::{Engine, Format};
use fetiche_formats::{Asd, DronePoint};

#[tracing::instrument]
pub fn import_data(_engine: &Engine, data: &str, _fmt: Format) -> Result<()> {
    trace!("import_data");

    // Transform into our `Drone` struct and sort it by "journey"
    //
    let data: Vec<Asd> = serde_json::from_str(data)?;

    let _journeys = BTreeMap::<u32, Vec<DronePoint>>::new();

    // Convert everything into list of `DronePoint` and insert by journey
    //
    // data.iter()
    //     .map(|asd| {
    //         let d = DronePoint::from(asd);
    //         (d.journey, d)
    //     })
    //     .for_each(|(j, d)| {
    //         // Retrieve the current list of points if journey is already known
    //         //
    //         let list = match journeys.get_mut(&j) {
    //             // It does
    //             //
    //             Some(list) => {
    //                 list.push(d);
    //                 list
    //             }
    //             // No record yet
    //             //
    //             _ => {
    //                 vec![d]
    //             }
    //         };
    //         journeys.insert(j, list.to_owned())
    //     });

    info!("{} journey points found.", data.len());
    Ok(())
}
