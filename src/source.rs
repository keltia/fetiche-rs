//! Definition of a data source
//!

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Source {
    /// ANY address
    pub base_url: String,
    /// Login to ANY server
    pub login: String,
    /// Password to ANY server
    pub password: String,
}

impl Default for Source {
    fn default() -> Self {
        Source::new()
    }
}

impl Source {
    /// Create a new empty source
    pub fn new() -> Self {
        Source {
            base_url: "http://127.0.0.1:2400/".to_string(),
            login: "USERNAME".to_string(),
            password: "PASSWORD".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_default() {
        let s = Source::new();

        assert_eq!("USERNAME", s.login);
    }
}
