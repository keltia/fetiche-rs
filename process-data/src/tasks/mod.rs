pub use export::*;
pub use setup::*;
pub use to_home::*;
pub use to_planes::*;

mod export;
mod setup;
mod to_home;
mod to_planes;
mod various;

/// One degree in *kilometers*
const ONE_DEG: f64 = 40_000. / 360.;
