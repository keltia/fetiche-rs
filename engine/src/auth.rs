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
#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
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

impl Debug for Auth {
    /// Prints the authentication method with all credential fields redacted.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let method = match self {
            Self::Anon => return f.write_str("Anon"),
            Self::UserKey { .. } => "UserKey",
            Self::Key { .. } => "Key",
            Self::Token { .. } => "Token",
            Self::Vhost { .. } => "Vhost",
            Self::Login { .. } => "Login",
        };
        write!(f, "{method}(HIDDEN)")
    }
}

impl Display for Auth {
    /// Displays the authentication method without credential values.
    ///
    ///
    /// ```
    /// use fetiche_engine::Auth;
    ///
    /// let auth = Auth::Key { api_key: "example-key".to_owned() };
    /// assert_eq!(auth.to_string(), "Key(HIDDEN)");
    /// ```
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
    AsdToken,
}
