//! Statistics manipulation module
//!

use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::vec;

use chrono::{DateTime, Utc};
use itertools::fold;

/// All different statistics
///
#[derive(Clone, Debug)]
pub enum Stats {
    Planes(PlanesStats),
}

impl Stats {
    /// Gather all instances stats into one
    ///
    pub fn summarise(v: Vec<Stats>) -> Stats {
        match v.len() {
            0 => return Stats::Planes(PlanesStats::default()),
            _ => {
                let first = v[0].clone();
                if v.len() == 1 {
                    first
                } else {
                    fold(v[1..].iter(), first, |a, b| a + b.clone())
                }
            }
        }
    }
}

impl Add for Stats {
    type Output = Self;

    /// Add two statistics
    ///
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Stats::Planes(inner) => {
                Stats::Planes(inner + rhs.into())
            }
        }
    }
}

// -----

/// Dereference `Stats` into inner `PlaneStats`
///
impl From<Stats> for PlanesStats {
    fn from(value: Stats) -> Self {
        match value {
            Stats::Planes(inner) => inner,
        }
    }
}

// -----

#[derive(Clone, Debug)]
pub struct PlanesStats {
    /// Specific date
    day: Vec<DateTime<Utc>>,
    /// Number of plane points
    pub planes: usize,
    /// Number of drone points
    pub drones: usize,
    /// Number of potential encounters
    pub potential: usize,
    /// Effective number of encounters after calculations
    pub encounters: usize,
    /// Distance used for calculations
    distance: f64,
    /// Proximity used for calculations
    proximity: f64,
    /// Time for processing in ms
    pub time: u128,
}

impl Default for PlanesStats {
    fn default() -> Self {
        Self {
            day: vec![Utc::now()],
            distance: 0.,
            proximity: 0.,
            planes: 0,
            drones: 0,
            potential: 0,
            encounters: 0,
            time: 0,
        }
    }
}

impl PlanesStats {
    pub fn new(day: DateTime<Utc>, distance: f64, proximity: f64) -> Self {
        Self {
            day: vec![day],
            distance,
            proximity,
            ..Self::default()
        }
    }
}

impl Display for PlanesStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = format!("{} drones in potential airprox with {} planes, {} found within {}m in a {} nm radius.\n\
        Time spent: {} ms\n",
                          self.drones, self.planes, self.encounters, self.proximity, self.distance, self.time);
        write!(f, "{}", str)
    }
}

impl Add for PlanesStats {
    type Output = PlanesStats;

    fn add(self, rhs: Self) -> Self::Output {
        let mut days = self.day.clone();
        let added = rhs.day.first().unwrap().to_owned();
        days.push(added);
        Self {
            day: days.clone(),
            distance: self.distance,
            proximity: self.proximity,
            planes: self.planes + rhs.planes,
            drones: self.drones + rhs.drones,
            potential: self.potential + rhs.potential,
            encounters: self.encounters + rhs.encounters,
            time: self.time + rhs.time,
        }
    }
}
