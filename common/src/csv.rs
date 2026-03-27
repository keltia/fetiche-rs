//! Generic version of prepare_csv() from fetiche-formats.
//!
//! This module provides utilities for converting serializable data structures into CSV format
//! with support for different delimiters.
//!
//! # Examples
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {}
//! use fetiche_common::csv::Delim;
//! use serde::Serialize;
//!
//! #[derive(Debug, Serialize)]
//! struct Record {
//!     name: String,
//!     age: u32,
//! }
//!
//! let data = vec![
//!     Record { name: "Alice".to_string(), age: 30 },
//!     Record { name: "Bob".to_string(), age: 25 },
//! ];
//!
//! let csv = Delim::Comma.prepare_csv(&data, true)?;
//! assert!(csv.contains("name,age"));
//! assert!(csv.contains("Alice,30"));
//! assert!(csv.contains("Bob,25"));
//! # Ok(())
//! # }
//! ```
//!
use std::fmt::Debug;

use csv::WriterBuilder;
use eyre::Result;
use serde::Serialize;

/// Represents the delimiter type to use when generating CSV output.
///
#[derive(Debug, Copy, Clone)]
pub enum Delim {
    /// Colon delimiter (`:`)
    Colon,
    /// Comma delimiter (`,`) - standard CSV format
    Comma,
    /// Tab delimiter (`\t`) - TSV format
    Tab,
}

impl Delim {
    /// Converts a vector of serializable data into a CSV-formatted string.
    ///
    /// # Arguments
    /// * `data` - A reference to a vector of items to be serialized as CSV
    /// * `header` - Whether to include a header row in the output
    ///
    /// # Returns
    /// Returns a `Result<String>` containing the CSV-formatted data on success.
    ///
    /// # Errors
    /// This function will return an error if:
    /// * CSV serialization fails
    /// * The output cannot be converted to a valid UTF-8 string
    ///
    /// # Examples
    /// ```
    /// use fetiche_common::csv::Delim;
    /// use serde::Serialize;
    ///
    /// #[derive(Debug, Serialize)]
    /// struct Person {
    ///     name: String,
    ///     age: u32,
    /// }
    ///
    /// let people = vec![
    ///     Person { name: "Alice".to_string(), age: 30 },
    ///     Person { name: "Bob".to_string(), age: 25 },
    /// ];
    ///
    /// let csv = Delim::Comma.prepare_csv(&people, true).unwrap();
    /// assert!(csv.contains("name,age"));
    /// assert!(csv.contains("Alice,30"));
    /// ```
    ///
    #[tracing::instrument(skip(self, data))]
    pub fn prepare_csv<T>(&self, data: &Vec<T>, header: bool) -> Result<String>
    where
        T: Serialize + Debug,
    {
        let delim = match self {
            Self::Comma => b',',
            Self::Colon => b':',
            Self::Tab => b'\t',
        };
        let mut wtr = WriterBuilder::new()
            .delimiter(delim)
            .has_headers(header)
            .from_writer(vec![]);

        data.iter().for_each(|rec| {
            wtr.serialize(rec).unwrap();
        });

        let data = String::from_utf8(wtr.into_inner()?)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    struct TestRecord {
        name: String,
        value: i32,
    }

    #[test]
    fn test_comma_delimiter_with_header() {
        let data = vec![
            TestRecord {
                name: "foo".to_string(),
                value: 42,
            },
            TestRecord {
                name: "bar".to_string(),
                value: 100,
            },
        ];

        let result = Delim::Comma.prepare_csv(&data, true).unwrap();
        assert!(result.contains("name,value"));
        assert!(result.contains("foo,42"));
        assert!(result.contains("bar,100"));
    }

    #[test]
    fn test_comma_delimiter_without_header() {
        let data = vec![
            TestRecord {
                name: "foo".to_string(),
                value: 42,
            },
            TestRecord {
                name: "bar".to_string(),
                value: 100,
            },
        ];

        let result = Delim::Comma.prepare_csv(&data, false).unwrap();
        assert!(!result.contains("name,value"));
        assert!(result.contains("foo,42"));
        assert!(result.contains("bar,100"));
    }

    #[test]
    fn test_colon_delimiter() {
        let data = vec![
            TestRecord {
                name: "foo".to_string(),
                value: 42,
            },
            TestRecord {
                name: "bar".to_string(),
                value: 100,
            },
        ];

        let result = Delim::Colon.prepare_csv(&data, true).unwrap();
        assert!(result.contains("name:value"));
        assert!(result.contains("foo:42"));
        assert!(result.contains("bar:100"));
    }

    #[test]
    fn test_tab_delimiter() {
        let data = vec![
            TestRecord {
                name: "foo".to_string(),
                value: 42,
            },
            TestRecord {
                name: "bar".to_string(),
                value: 100,
            },
        ];

        let result = Delim::Tab.prepare_csv(&data, true).unwrap();
        assert!(result.contains("name\tvalue"));
        assert!(result.contains("foo\t42"));
        assert!(result.contains("bar\t100"));
    }

    #[test]
    fn test_empty_data() {
        let data: Vec<TestRecord> = vec![];
        let result = Delim::Comma.prepare_csv(&data, true).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_record() {
        let data = vec![TestRecord {
            name: "single".to_string(),
            value: 99,
        }];

        let result = Delim::Comma.prepare_csv(&data, true).unwrap();
        assert!(result.contains("name,value"));
        assert!(result.contains("single,99"));
    }

    #[test]
    fn test_special_characters_in_data() {
        let data = vec![TestRecord {
            name: "foo,bar".to_string(),
            value: 42,
        }];

        let result = Delim::Comma.prepare_csv(&data, true).unwrap();
        // CSV should quote fields with special characters
        //
        assert!(result.contains("\"foo,bar\""));
    }

    #[test]
    fn test_delim_clone_and_copy() {
        let delim1 = Delim::Comma;
        let delim2 = delim1;
        let delim3 = delim1.clone();

        let data = vec![TestRecord {
            name: "test".to_string(),
            value: 1,
        }];

        let result1 = delim1.prepare_csv(&data, true).unwrap();
        let result2 = delim2.prepare_csv(&data, true).unwrap();
        let result3 = delim3.prepare_csv(&data, true).unwrap();

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_delim_debug() {
        let delim = Delim::Comma;
        let debug_str = format!("{:?}", delim);
        assert_eq!(debug_str, "Comma");
    }
}
