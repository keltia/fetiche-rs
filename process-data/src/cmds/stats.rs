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
            0 => Stats::Planes(PlanesStats::default()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_planes_stats_default() {
        let stats = PlanesStats::default();
        assert_eq!(stats.planes, 0);
        assert_eq!(stats.drones, 0);
        assert_eq!(stats.potential, 0);
        assert_eq!(stats.encounters, 0);
        assert_eq!(stats.time, 0);
        assert_eq!(stats.distance, 0.0);
        assert_eq!(stats.proximity, 0.0);
        assert!(stats.day.len() == 1);
    }

    #[test]
    fn test_planes_stats_new() {
        let now = Utc::now();
        let distance = 50.0;
        let proximity = 10.0;
        let stats = PlanesStats::new(now, distance, proximity);

        assert_eq!(stats.planes, 0);
        assert_eq!(stats.drones, 0);
        assert_eq!(stats.potential, 0);
        assert_eq!(stats.encounters, 0);
        assert_eq!(stats.time, 0);
        assert_eq!(stats.distance, distance);
        assert_eq!(stats.proximity, proximity);
        assert!(stats.day.contains(&now));
    }

    #[test]
    fn test_planes_stats_add() {
        let day1 = Utc::now();
        let stats1 = PlanesStats {
            day: vec![day1],
            distance: 50.0,
            proximity: 10.0,
            planes: 5,
            drones: 7,
            potential: 3,
            encounters: 2,
            time: 100,
        };

        let day2 = Utc::now();
        let stats2 = PlanesStats {
            day: vec![day2],
            distance: 50.0,
            proximity: 10.0,
            planes: 10,
            drones: 8,
            potential: 4,
            encounters: 3,
            time: 150,
        };

        let result = stats1 + stats2;
        assert_eq!(result.planes, 15);
        assert_eq!(result.drones, 15);
        assert_eq!(result.potential, 7);
        assert_eq!(result.encounters, 5);
        assert_eq!(result.time, 250);
        assert_eq!(result.distance, 50.0);
        assert_eq!(result.proximity, 10.0);
        assert!(result.day.len() == 2);
        assert!(result.day.contains(&day1));
        assert!(result.day.contains(&day2));
    }

    #[test]
    fn test_stats_summarise_empty() {
        let summarised = Stats::summarise(vec![]);
        if let Stats::Planes(planes_stats) = summarised {
            assert_eq!(planes_stats.planes, 0);
            assert_eq!(planes_stats.drones, 0);
            assert_eq!(planes_stats.potential, 0);
            assert_eq!(planes_stats.encounters, 0);
            assert_eq!(planes_stats.time, 0);
            assert_eq!(planes_stats.distance, 0.0);
            assert_eq!(planes_stats.proximity, 0.0);
            assert!(planes_stats.day.len() == 1);
        }
    }

    #[test]
    fn test_stats_summarise_non_empty() {
        let now = Utc::now();
        let stats1 = Stats::Planes(PlanesStats::new(now, 50.0, 10.0));
        let stats2 = Stats::Planes(PlanesStats {
            day: vec![now],
            distance: 50.0,
            proximity: 10.0,
            planes: 10,
            drones: 8,
            potential: 4,
            encounters: 3,
            time: 150,
        });

        let summarised = Stats::summarise(vec![stats1.clone(), stats2.clone()]);
        if let Stats::Planes(planes_stats) = summarised {
            assert!(planes_stats.day.len() >= 2);
        }
    }
}

