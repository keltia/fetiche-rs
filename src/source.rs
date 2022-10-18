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
        Source {
            base_url: "http://127.0.0.1:2400/".to_string(),
            login: "USERNAME".to_string(),
            password: "PASSWORD".to_string(),
        }
    }
}
