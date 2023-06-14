use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

/// Describe the possible ways to authenticate oneself
///
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Auth {
    /// Nothing special, no auth
    #[default]
    Anon,
    /// Using an API key supplied through the URL or a header
    Key { api_key: String },
    /// Using a login/passwd to get a token
    Token {
        login: String,
        password: String,
        token: String,
    },
    /// Using plain login/password
    Login { username: String, password: String },
}

impl Display for Auth {
    /// Obfuscate the passwords & keys
    ///
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Hide passwords & API keys
        //
        //let auth = self.clone();
        let auth = match self.clone() {
            Auth::Key { .. } => Auth::Key {
                api_key: "HIDDEN".to_string(),
            },
            Auth::Login { username, .. } => Auth::Login {
                username,
                password: "HIDDEN".to_string(),
            },
            Auth::Token { login, token, .. } => Auth::Token {
                login,
                token,
                password: "HIDDEN".to_string(),
            },
            _ => Auth::Anon,
        };
        write!(f, "{:?}", auth)
    }
}
