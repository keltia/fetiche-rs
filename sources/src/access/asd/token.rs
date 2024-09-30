use crate::Expirable;
use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Access token derived from username/password
///
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsdToken {
    /// The actual token
    pub token: String,
    /// Don't ask
    gjrt: String,
    /// Expiration date
    pub expired_at: i64,
    roles: Vec<String>,
    /// Fullname
    name: String,
    supervision: Option<String>,
    lang: String,
    status: String,
    email: String,
    airspace_admin: Option<String>,
    homepage: String,
}

impl AsdToken {
    pub fn export(&self) -> Result<String> {
        Ok(json!(&self).to_string())
    }
}

impl Expirable for AsdToken {
    #[inline]
    fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expired_at
    }

    #[inline]
    fn key(&self) -> String {
        self.email.clone()
    }
}

impl Default for AsdToken {
    fn default() -> Self {
        AsdToken {
            token: "".to_owned(),
            gjrt: "".to_owned(),
            expired_at: 0i64,
            roles: vec![],
            name: "John Doe".to_owned(),
            supervision: None,
            lang: "en".to_owned(),
            status: "".to_owned(),
            email: "john.doe@example.net".to_owned(),
            airspace_admin: None,
            homepage: "https://example.net".to_owned(),
        }
    }
}
