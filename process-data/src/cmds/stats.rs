use std::fmt::{Display, Formatter};

use chrono::{DateTime, Utc};

/// Every time we run a calculation for any given day, we store the statistics for the run.
///
#[derive(Debug, Default)]
pub enum Stats {
    /// Statistics for the plane to drone calculations.
    Planes {
        /// Specific date
        day: DateTime<Utc>,
        /// Number of plane points
        planes: usize,
        /// Number of drone points
        drones: usize,
        /// Number of potential encounters
        potential: usize,
        /// Effective number of encounters after calculations
        encounters: usize,
        /// Distance used for calculations
        distance: f64,
        /// Proximity used for calculations
        proximity: f64,
    },
    /// Statistics for the home to drone calculations.
    Home { distances: usize },
    #[default]
    None,
}

impl Display for Stats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Self::Home { distances } => {
                format!(
                    "Calculated {} distances between drone and operator.",
                    distances
                )
            }
            Self::Planes {
                day,
                planes,
                drones,
                encounters,
                distance,
                proximity,
                ..
            } => {
                format!("Day {}:\n{} drones in potential airprox with {} planes, {} found within {}m in a {} nm radius.",
                        day, drones, planes, encounters, proximity, distance)
            }
            _ => String::from(""),
        };
        write!(f, "{}", str)
    }
}
