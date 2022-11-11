//! submodule to manage date filters
//!

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// If we specify -B/-E or --today, we need to pass these below
///
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

    /// Serialize into json to pass around as a String
    ///
    pub fn to_string(&self) -> String {
        #[derive(Debug, Serialize)]
        struct Minimal {
            begin: NaiveDateTime,
            end: NaiveDateTime,
        }

        match self {
            Filter::None => "{}".to_owned(),
            Filter::Interval { begin, end } => {
                let m = Minimal {
                    begin: *begin,
                    end: *end,
                };
                json!(m).to_string()
            }
        }
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

    #[test]
    fn test_filter_to_string() {
        let begin = "2022-11-11 12:34:56";
        let end = "2022-11-30 12:34:56";

        let begin = NaiveDateTime::parse_from_str(begin, "%Y-%m-%d %H:%M:%S");
        assert!(begin.is_ok());
        let end = NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S");
        assert!(end.is_ok());

        let r = r##"{"begin":"2022-11-11T12:34:56","end":"2022-11-30T12:34:56"}"##;

        let f = Filter::from(begin.unwrap(), end.unwrap());
        let s = f.to_string();
        assert_eq!(r, &s);
    }
}
