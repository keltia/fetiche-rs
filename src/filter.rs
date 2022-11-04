//! submodule to manage date filters
//!

use chrono::NaiveDateTime;

/// If we specify -B/-E or --today, we need to pass these below
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Filter {
    Interval {
        begin: NaiveDateTime,
        end: NaiveDateTime,
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
    pub fn from(begin: NaiveDateTime, end: NaiveDateTime) -> Self {
        Filter::Interval { begin, end }
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

        let begin = NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S");
        assert!(begin.is_ok());
        let end = NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S");
        assert!(end.is_ok());

        let f = Filter::from(begin.unwrap(), end.unwrap());
        assert_ne!(Filter::None, f);
        Ok(())
    }
}
