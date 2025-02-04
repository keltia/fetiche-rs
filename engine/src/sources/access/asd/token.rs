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

use chrono::{Days, Utc};
use eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::trace;

use crate::Expirable;

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
    /// Not documented
    roles: Vec<String>,
    /// Fullname
    name: String,
    /// Not documented
    supervision: Option<String>,
    lang: String,
    status: String,
    email: String,
    /// Not documented
    airspace_admin: Option<String>,
    /// Not documented
    homepage: String,
}

impl AsdToken {
    #[tracing::instrument]
    /// Return an invalid (and expired) token by default.
    ///
    pub fn new() -> Self {
        let d = Utc::now().checked_sub_days(Days::new(1)).unwrap();
        AsdToken {
            token: "INVALID".into(),
            gjrt: "INVALID".into(),
            expired_at: d.timestamp(),
            roles: vec![],
            name: "INVALID".into(),
            supervision: None,
            lang: "en".into(),
            status: "INVALID".into(),
            email: "INVALID".into(),
            airspace_admin: None,
            homepage: "INVALID".into(),
        }
    }

    #[tracing::instrument]
    pub fn from_json(json: &str) -> Result<Self> {
        let temp: AsdToken = serde_json::from_str(json).unwrap_or(AsdToken::new());
        Ok(temp)
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
    pub fn retrieve(fname: &PathBuf) -> Result<AsdToken> {
        let str = if fname.exists() {
            fs::read_to_string(fname)?
        } else {
            "INVALID".into()
        };
        Ok(AsdToken::from_json(&str)?)
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
    #[tracing::instrument]
    fn default() -> Self {
        Self::new()
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

