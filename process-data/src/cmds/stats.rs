//! Statistics manipulation module
//!

use std::fmt::{Display, Formatter};
use std::ops::Add;

use chrono::{DateTime, Utc};
use itertools::fold;

/// All different statistics
///
#[derive(Clone, Copy, Debug)]
pub enum Stats {
    Planes(PlanesStats),
    Home(HomeStats),
}

impl Stats {
    /// Gather all instances stats into one
    ///
    pub fn summarise(v: Vec<Stats>) -> Stats {
        let first = v[0].clone();
        if v.len() == 1 {
            first
        } else {
            fold(v[1..].into_iter(), first, |a, &b| a + b)
        }
    }
}

impl Add for Stats {
    type Output = Self;

    /// Add two statistics
    ///
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Stats::Home(inner) => {
                Stats::Home(inner + rhs.into())
            }
            Stats::Planes(inner) => {
                Stats::Planes(inner + rhs.into())
            }
        }
    }
}

/// Dereference `Stats` into inner `HomeStats`
///
impl From<Stats> for HomeStats {
    fn from(value: Stats) -> Self {
        match value {
            Stats::Home(inner) => inner,
            _ => panic!("bad type")
        }
    }
}

/// Dereference `Stats` into inner `PlaneStats`
///
impl From<Stats> for PlanesStats {
    fn from(value: Stats) -> Self {
        match value {
            Stats::Planes(inner) => inner,
            _ => panic!("bad type")
        }
    }
}

// -----

#[derive(Clone, Copy, Debug)]
pub struct PlanesStats {
    /// Specific date
    day: DateTime<Utc>,
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
}

impl Default for PlanesStats {
    fn default() -> Self {
        Self {
            day: Utc::now(),
            distance: 0.,
            proximity: 0.,
            planes: 0,
            drones: 0,
            potential: 0,
            encounters: 0,
        }
    }
}

impl PlanesStats {
    pub fn new(day: DateTime<Utc>, distance: f64, proximity: f64) -> Self {
        Self {
            day,
            distance,
            proximity,
            ..Self::default()
        }
    }
}

impl Display for PlanesStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = format!("Day {}:\n{} drones in potential airprox with {} planes, {} found within {}m in a {} nm radius.",
                          self.day, self.drones, self.planes, self.encounters, self.proximity, self.distance);
        write!(f, "{}", str)
    }
}

impl Add for PlanesStats {
    type Output = PlanesStats;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            day: self.day,
            distance: self.distance,
            proximity: self.proximity,
            planes: self.planes + rhs.planes,
            drones: self.drones + rhs.drones,
            potential: self.potential + rhs.potential,
            encounters: self.encounters + rhs.encounters,
        }
    }
}

// -----

#[derive(Clone, Copy, Debug)]
pub struct HomeStats {
    /// Statistics for the home to drone calculations.
    pub distances: usize,
}

impl Default for HomeStats {
    fn default() -> Self {
        Self::new()
    }
}

impl HomeStats {
    pub fn new() -> Self {
        Self {
            distances: 0,
        }
    }
}

impl Display for HomeStats {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = format!("Calculated {} distances between drone and operator.", self.distances);
        write!(f, "{}", str)
    }
}

impl Add for HomeStats {
    type Output = HomeStats;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            distances: self.distances + rhs.distances,
        }
    }
}

