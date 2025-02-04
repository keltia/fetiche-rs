//! Statistics manipulation module
//!

use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::vec;

use chrono::{DateTime, Utc};
use itertools::fold;

/// Represents different types of statistics that can be gathered and manipulated.
///
/// Currently, this enum supports only `Planes` statistics which encapsulate
/// all the information about planes, drones, and their encounters.
///
#[derive(Clone, Debug)]
pub enum Stats {
    Planes(PlanesStats),
}

impl Stats {
    /// Summarize a vector of `Stats` into a single `Stats` instance.
    ///
    /// This method takes a vector of `Stats` and combines them into one.
    /// If the vector is empty, a default instance of `Stats::Planes` is returned.
    /// If there is only one element in the vector, it is returned directly.
    /// Otherwise, all the elements in the vector are folded together using the `+` operator.
    ///
    /// # Arguments
    ///
    /// * `v` - A vector of `Stats` to be summarized.
    ///
    /// # Returns
    ///
    /// A single combined `Stats` instance that represents the summary of all the input elements.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use chrono::Utc;
    /// let stats1 = Stats::Planes(PlanesStats::new(Utc::now(), 50.0, 10.0));
    /// let stats2 = Stats::Planes(PlanesStats::new(Utc::now(), 60.0, 15.0));
    ///
    /// let summary = Stats::summarise(vec![stats1, stats2]);
    ///
    /// // The `summary` will now represent the combined statistics.
    /// ```
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

/// Performs addition between two `Stats` objects.
///
/// This implementation ensures that when two `Stats` of the same type
/// (currently only `Stats::Planes`) are added together, their respective
/// statistics are summed. For additional types of `Stats` this method
/// would need to be extended accordingly.
///
/// # Arguments
///
/// * `self` - The left-hand operand in the addition operation.
/// * `rhs` - The right-hand operand in the addition operation.
///
/// # Returns
///
/// A new `Stats` object representing the combined result of adding
/// the two input objects.
///
/// # Example
///
/// ```rust,no_run
/// let stats1 = Stats::Planes(PlanesStats::new(Utc::now(), 50.0, 10.0));
/// let stats2 = Stats::Planes(PlanesStats::new(Utc::now(), 60.0, 15.0));
///
/// let combined = stats1 + stats2;
///
/// match combined {
///     Stats::Planes(inner) => {
///         assert_eq!(inner.planes, stats1.into().planes + stats2.into().planes);
///         assert_eq!(inner.drones, stats1.into().drones + stats2.into().drones);
///     }
/// }
/// ```
///
impl Add for Stats {
    type Output = Self;

    /// Add two statistics
    ///
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Stats::Planes(inner) => Stats::Planes(inner + rhs.into()),
        }
    }
}

// -----

/// Converts a `Stats` enum instance into a `PlanesStats` struct instance.
///
/// This implementation ensures that only `Stats::Planes` variant can be converted.
/// Attempting to convert other variants will result in a compile-time error.
///
/// # Arguments
///
/// * `value` - The `Stats` enum instance to convert.
///
/// # Returns
///
/// A `PlanesStats` struct instance that encapsulates the data
/// contained in the `Stats::Planes` variant.
///
/// # Example
///
/// ```rust,no_run
/// let stats = Stats::Planes(PlanesStats::new(Utc::now(), 100.0, 5.0));
/// let planes_stats: PlanesStats = stats.into();
///
/// assert_eq!(planes_stats.planes, 0);
/// ```
///
impl From<Stats> for PlanesStats {
    fn from(value: Stats) -> Self {
        match value {
            Stats::Planes(inner) => inner,
        }
    }
}

// -----

/// Represents statistics relating to planes, drones, and potential encounters.
///
/// This structure encapsulates various metrics derived from calculations and
/// monitoring of planes and drones in a specific area around a time frame.
///
/// # Fields
///
/// * `day` - A vector of dates associated with the statistics data.
/// * `planes` - The number of planes involved in the monitoring.
/// * `drones` - The number of drones detected in the monitored area.
/// * `potential` - The estimated number of potential encounters between planes and drones.
/// * `encounters` - The actual number of encounters determined after processing.
/// * `distance` - The distance radius around which the calculations are performed, in nautical miles.
/// * `proximity` - The proximity threshold used for encounter detection, in meters.
/// * `time` - The processing time spent calculating statistics, measured in milliseconds.
///
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

/// Provides a default implementation for creating a new `PlanesStats` instance.
///
/// # Details
///
/// The default implementation sets all fields of the `PlanesStats` structure to their
/// initial values:
///
/// * `day`: A vector containing the current UTC time.
/// * `planes`: 0 (no planes detected by default).
/// * `drones`: 0 (no drones detected by default).
/// * `potential`: 0 (no potential encounters by default).
/// * `encounters`: 0 (no actual encounters by default).
/// * `distance`: 0.0 (no measurable distance set by default).
/// * `proximity`: 0.0 (no proximity threshold set by default).
/// * `time`: 0 (no time spent calculating by default).
///
/// This implementation is useful for creating an empty statistics object to update or
/// use in further computations.
///
/// # Example
///
/// ```rust
/// let stats = PlanesStats::default();
///
/// assert_eq!(stats.planes, 0);
/// assert_eq!(stats.drones, 0);
/// assert_eq!(stats.potential, 0);
/// assert_eq!(stats.encounters, 0);
/// assert_eq!(stats.distance, 0.0);
/// assert_eq!(stats.proximity, 0.0);
/// assert_eq!(stats.time, 0);
/// ```
///
impl Default for PlanesStats {
    fn default() -> Self {
        Self {
            day: vec![Utc::now()],
            planes: 0,
            drones: 0,
            potential: 0,
            encounters: 0,
            distance: 0.0,
            proximity: 0.0,
            time: 0,
        }
    }
}

impl PlanesStats {
    /// Create a new instance of `PlanesStats`.
    ///
    /// # Arguments
    ///
    /// * `day` - A specific date associated with the statistics.
    /// * `distance` - The calculation radius in nautical miles.
    /// * `proximity` - The proximity threshold for encounter detection, in meters.
    ///
    /// # Returns
    ///
    /// A new instance of `PlanesStats` with the provided `day`, `distance`, and `proximity`,
    /// while other fields are initialized with their default values.
    ///
    /// # Example
    ///
    /// ```rust
    /// use chrono::Utc;
    ///
    /// let day = Utc::now();
    /// let stats = PlanesStats::new(day, 100.0, 5.0);
    ///
    /// assert_eq!(stats.day.len(), 1);
    /// assert_eq!(stats.day[0], day);
    /// assert_eq!(stats.distance, 100.0);
    /// assert_eq!(stats.proximity, 5.0);
    /// assert_eq!(stats.planes, 0);
    /// assert_eq!(stats.drones, 0);
    /// assert_eq!(stats.potential, 0);
    /// assert_eq!(stats.encounters, 0);
    /// assert_eq!(stats.time, 0);
    /// ```
    ///
    #[inline]
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

/// Add two `PlanesStats` instances together.
///
/// This operation combines the statistical data from two `PlanesStats`
/// instances, aggregating the number of planes, drones, potential encounters,
/// actual encounters, and processing time. It also merges the `day` field,
/// appending the date from the `rhs` instance.
///
/// # Arguments
///
/// * `self` - The first `PlanesStats` instance.
/// * `rhs` - The second `PlanesStats` instance to add to the first.
///
/// # Returns
///
/// A new `PlanesStats` instance containing the aggregated stats from both inputs.
///
/// # Example
///
/// ```rust
/// let day1 = Utc::now();
/// let stats1 = PlanesStats {
///     day: vec![day1],
///     distance: 50.0,
///     proximity: 10.0,
///     planes: 5,
///     drones: 7,
///     potential: 3,
///     encounters: 2,
///     time: 100,
/// };
///
/// let day2 = Utc::now();
/// let stats2 = PlanesStats {
///     day: vec![day2],
///     distance: 50.0,
///     proximity: 10.0,
///     planes: 10,
///     drones: 8,
///     potential: 4,
///     encounters: 3,
///     time: 150,
/// };
///
/// let result = stats1 + stats2;
///
/// assert_eq!(result.planes, 15);
/// assert_eq!(result.drones, 15);
/// assert_eq!(result.potential, 7);
/// assert_eq!(result.encounters, 5);
/// assert_eq!(result.time, 250);
/// assert_eq!(result.distance, 50.0);
/// assert_eq!(result.proximity, 10.0);
/// assert!(result.day.len() == 2);
/// assert!(result.day.contains(&day1));
/// assert!(result.day.contains(&day2));
/// ```
///
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

    #[allow(irrefutable_let_patterns)]
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

    #[allow(irrefutable_let_patterns)]
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
