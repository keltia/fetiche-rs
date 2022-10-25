//! submodule to manage date filters
//!

use chrono::{DateTime, Datelike, Local, TimeZone};

use crate::cli::Opts;

/// If we specify -B/-E or --today, we need to pass these below
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Filter {
    Interval {
        begin: DateTime<Local>,
        end: DateTime<Local>,
    },
    None,
}

impl Default for Filter {
    /// Defaults to nothing
    ///
    fn default() -> Self {
        Filter::None
    }
}

impl Filter {
    /// from two time points
    ///
    pub fn from(begin: DateTime<Local>, end: DateTime<Local>) -> Self {
        Filter::Interval { begin, end }
    }

    /// From the CLI options
    ///
    pub fn from_opts(opts: &Opts) -> Self {
        let t: DateTime<Local> = Local::now();

        let filter = if opts.today {
            // Build our own begin, end
            //
            let begin = Local.ymd(t.year(), t.month(), t.day()).and_hms(0, 0, 0);
            let end = Local.ymd(t.year(), t.month(), t.day()).and_hms(23, 59, 59);

            Filter::from(begin, end)
        } else if opts.begin.is_some() {
            // Assume both are there, checked elsewhere
            //
            Filter::from(opts.begin.unwrap(), opts.end.unwrap())
        } else {
            Filter::default()
        };
        filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_filter_new() -> Result<()> {
        let begin = "2022-11-11 12:34:56";
        let end = "2022-11-30 12:34:56";

        let begin = begin.parse::<DateTime<Local>>()?;
        let end = end.parse::<DateTime<Local>>()?;

        let f = Filter::from(begin, end);
        assert_ne!(Filter::None, f);
        Ok(())
    }
}
