//! Module for managing ASD tokens including storage, retrieval, and purge operations.
//!
//! This module defines the `AsdToken` structure which encapsulates token data, 
//! expiration information, and user profile details such as email, roles, and homepage.
//! It includes methods to export token details into JSON format and provides default values.
//!
//! Additionally, this module includes functionality within the `Asd` implementation for:
//!
//! - Retrieving token data from a file.
//! - Storing token data into a file, creating necessary directories if required.
//! - Purging expired tokens by removing their associated files.
//!
//! The `Expirable` trait is implemented for `AsdToken` to facilitate checking token expiration.
//! Utilities of this module are designed to handle errors gracefully using `eyre::Result` and
//! tracing instrumentation for debugging.
//!
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::trace;

use crate::{Asd, AuthError, Expirable};

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

impl AsdToken {
    /// Exports the token details into a JSON string format.
    ///
    /// The `export` method serializes the `AsdToken` instance into a JSON string
    /// representation using the `serde_json` crate.
    ///
    /// # Returns
    ///
    /// - `Ok(String)`: A JSON string representing the `AsdToken` instance.
    /// - `Err(eyre::Report)`: An error if the serialization fails unexpectedly.
    ///
    /// # Examples
    ///
    /// ```
    /// use fetiche_sources::AsdToken;
    ///
    /// let token = AsdToken::default();
    /// let result = token.export();
    ///
    /// assert!(result.is_ok());
    /// assert!(result.unwrap().contains("\"name\":\"John Doe\""));
    /// ```
    ///
    /// # Errors
    ///
    /// - Can return an `eyre::Report` if an internal error occurs during serialization.
    ///
    pub fn export(&self) -> Result<String> {
        Ok(json!(&self).to_string())
    }

    /// This function attempts to read the content of the specified token file.
    /// If the file exists, its content is returned as a `String`.
    /// If the file does not exist, an error of type `AuthError::Retrieval` is returned.
    ///
    /// # Parameters
    ///
    /// - `fname`: A reference to a `PathBuf` representing the path of the token file to be retrieved.
    ///
    /// # Returns
    ///
    /// - `Ok(String)`: The content of the token file if it exists.
    /// - `Err(eyre::Report)`: An error if the file does not exist or cannot be read.
    ///
    ///
    /// # Errors
    ///
    /// - Returns an error with the variant `AuthError::Retrieval` if the file cannot be found.
    /// - Returns an error if the file cannot be read due to insufficient permissions or other I/O issues.
    ///
    #[tracing::instrument]
    pub fn retrieve(fname: &PathBuf) -> Result<String> {
        if fname.exists() {
            Ok(fs::read_to_string(fname)?)
        } else {
            Err(AuthError::Retrieval(fname.to_string_lossy().to_string()).into())
        }
    }

    /// Store (overwrite) named token
    ///
    #[tracing::instrument]
    pub fn store(fname: &PathBuf, data: &str) -> Result<()> {
        let dir = fname.parent().unwrap();

        // Check token cache
        //
        if !dir.exists() {
            // Create it
            //
            trace!("create token store: {dir:?}");

            fs::create_dir_all(dir)?
        }
        trace!("store_token: {fname:?}");
        Ok(fs::write(fname, data)?)
    }

    /// Purge expired token
    ///
    #[tracing::instrument]
    pub fn purge(fname: &PathBuf) -> Result<()> {
        trace!("purge expired token in {fname:?}");

        Ok(fs::remove_file(fname)?)
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
