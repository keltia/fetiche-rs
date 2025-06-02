//! Everything related to authentication & tokens.
//!

use std::fmt::{Debug, Display, Formatter};

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::token::AsdToken;

/// Represents the different authentication mechanisms available for accessing resources.
///
/// The `Auth` enum provides multiple ways to authenticate, such as using API keys,
/// token-based authentication, or plain login credentials. Each variant corresponds
/// to a specific authentication method.
///
/// # Variants
///
/// * **Anon**
///   No authentication required.
///
/// * **UserKey**
///   Authentication using a combination of `api_key` and `user_key`.
///
/// * **Key**
///   Authentication using a single `api_key`, supplied either in the URL or as a header.
///
/// * **Token**
///   Authentication using a `login` and `password` to obtain a `token`.
///
/// * **Vhost**
///   Authentication using `vhost` (virtual host), `username`, and `password`.
///
/// * **Login**
///   Simple authentication using `username` and `password`.
///
/// # Example Usage:
///
/// ```rust
/// use fetiche_engine::Auth;
///
/// let auth = Auth::UserKey {
///     api_key: "my_api_key".to_string(),
///     user_key: "my_user_key".to_string(),
/// };
///
/// println!("Auth: {}", auth);
/// ```
///
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Auth {
    /// Nothing special, no auth
    #[default]
    Anon,
    /// API with both user-key and api-key
    UserKey { api_key: String, user_key: String },
    /// Using an API key supplied through the URL or a header
    Key { api_key: String },
    /// Using a login/passwd to get a token
    Token {
        login: String,
        password: String,
        token: String,
    },
    /// Using plain login/password inside a specific virtual host
    Vhost {
        vhost: String,
        username: String,
        password: String,
    },
    /// Using plain login/password
    Login { username: String, password: String },
}

impl Display for Auth {
    /// Implements the `Display` trait for the `Auth` enum to provide a user-friendly string representation.
    ///
    /// Sensitive information such as passwords and API keys are obfuscated for security purposes.
    ///
    /// # Obfuscation Details:
    ///
    /// - **Passwords:** Replaced with `"HIDDEN"`.
    /// - **API keys:** Replaced with `"HIDDEN"`.
    ///
    /// The representation depends on the variant of the `Auth` enum.
    ///
    /// # Examples:
    ///
    /// ```rust
    /// use fetiche_engine::Auth;
    ///
    /// let auth = Auth::Token {
    ///     login: "user".to_string(),
    ///     password: "mypassword".to_string(),
    ///     token: "12345".to_string(),
    /// };
    ///
    /// assert!(auth.to_string().contains("HIDDEN"));
    /// ```
    ///
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Hide passwords & API keys
        //
        let auth = match self.clone() {
            Auth::Vhost {
                vhost, username, ..
            } => Auth::Vhost {
                vhost,
                username,
                password: "HIDDEN".to_string(),
            },
            Auth::UserKey { .. } => Auth::UserKey {
                api_key: "HIDDEN".to_string(),
                user_key: "HIDDEN".to_string(),
            },
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

/// A trait representing an entity that holds a key and can expire.
///
/// The `Expirable` trait provides two essential methods:
/// - `key()`: Retrieves the unique identifier or "key" for the entity.
/// - `is_expired()`: Checks whether the entity is expired.
///
/// This trait can be used for managing credentials, tokens, or other
/// expirable resources.
///
/// # Example
///
///```rust
/// use serde::{Serialize, Deserialize};
/// use fetiche_engine::Expirable;
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MyToken {
///     key: String,
///     expiration: u64, // Epoch timestamp
/// }
///
/// impl Expirable for MyToken {
///     fn key(&self) -> String {
///         self.key.clone()
///     }
///
///     fn is_expired(&self) -> bool {
///         let current_time = 1681234567; // Example current timestamp
///         self.expiration < current_time
///     }
/// }
///
/// let token = MyToken {
///     key: String::from("my_unique_token"),
///     expiration: 1681234000,
/// };
///
/// println!("Token Key: {}", token.key());
/// println!("Is Expired: {}", token.is_expired());
/// ```
///
#[enum_dispatch(TokenType)]
pub trait Expirable {
    fn key(&self) -> String;
    fn is_expired(&self) -> bool;
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenType {
    AsdToken,
}
