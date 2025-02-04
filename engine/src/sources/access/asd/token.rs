//!
//! This module defines the `AsdToken` structure and its associated functionality
//! for managing access tokens derived from username/password authentication.
//!
//! It includes serialization and deserialization capabilities, expiry checks, 
//! as well as methods to read, store, export, and purge tokens.
//!
//! The module leverages various dependencies and Rust's standard library to 
//! provide a robust and flexible token management system, with emphasis on 
//! handling token expiration, file operations, and error reporting.
//!
use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::trace;

use crate::{AuthError, Expirable};

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
    /// This method saves the provided token data into a file at the specified
    /// path. If the directory structure leading to the file does not exist, it will
    /// be created automatically. The token data will overwrite any existing data
    /// in the file.
    ///
    /// # Parameters
    ///
    /// - `fname`: A reference to a `PathBuf` representing the path where the token
    ///   data should be stored.
    /// - `data`: A string slice containing the token data to be saved.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Returns an empty `Ok` result upon successfully storing the token.
    /// - `Err(eyre::Report)`: An error if the directory cannot be created or the
    ///   token cannot be written to the file.
    ///
    /// # Errors
    ///
    /// - Returns an error if the parent directory cannot be created or encountered
    ///   permission issues.
    /// - Returns an error if the token cannot be written to the file.
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

    /// Removes the specified token file from the file system.
    ///
    /// This method deletes the token file at the given path. It is primarily used
    /// to remove expired tokens to ensure they are no longer accessible.
    ///
    /// # Parameters
    ///
    /// - `fname`: A reference to a `PathBuf` representing the path of the token
    ///   file to be removed.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Returns an empty `Ok` result upon successfully deleting the token file.
    /// - `Err(eyre::Report)`: An error if the file cannot be deleted.
    ///
    /// # Errors
    ///
    /// - Returns an error if the file does not exist or cannot be removed due to
    ///   insufficient permissions or other I/O issues.
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
