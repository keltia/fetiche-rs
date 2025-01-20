use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::fs::read_dir;
use std::path::Path;
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use eyre::Result;
use fetiche_sources::{AsdToken, TokenType};
use tabled::builder::Builder;
use tabled::settings::Style;
use tracing::trace;

use crate::TokenStatus;

/// The `TokenStorage` struct provides functionality for managing tokens
/// stored as serialized files in a specified directory. It allows operations
/// such as registering the directory containing token files, storing tokens,
/// and retrieving tokens by key, as well as listing all tokens present in the
/// directory.
///
/// This struct is specifically designed to work with different types of tokens,
/// which are represented by the `TokenType` enum. The specific token format is
/// determined based on the file content.
///
/// # Fields
/// - `path`: A string representing the relative path to the directory containing token files.
/// - `list`: A `BTreeMap` storing the tokens. The keys are file names, and the values are the parsed `TokenType`s.
///
/// # Examples
///
/// ```rust
///
/// // Register a directory containing token files
/// use fetiche_engine::TokenStorage;
/// use fetiche_sources::{AsdToken, TokenType};
///
/// let mut storage = TokenStorage::register("path/to/tokens");
///
/// // Store a new token
/// let token_data = TokenType::AsdToken(AsdToken { /* ... */ });
/// storage.store("new_token_file", token_data);
///
/// // Retrieve a token by key
/// if let Ok(token) = storage.load("existing_token_file") {
///     println!("Loaded token: {:?}", token);
/// }
///
/// // List all tokens
/// if let Ok(token_list) = storage.list() {
///     println!("{}", token_list);
/// }
/// ```
///
#[derive(Debug)]
pub struct TokenStorage {
    /// `path` is relative to `root`.
    path: String,
    /// Btree of (key, AuthToken)
    list: BTreeMap<String, TokenType>,
}

impl TokenStorage {
    /// Read the directory and return all tokens (one per file)
    ///
    pub fn register(path: &str) -> Self {
        let mut db = BTreeMap::<String, TokenType>::new();
        if let Ok(dir) = read_dir(path) {
            dir.into_iter().for_each(|entry| {
                if let Ok(p) = entry {
                    let f = p.file_name().to_str().unwrap().to_string();
                    let full = Path::new(path).join(f.as_str());
                    let raw = fs::read_to_string(full).unwrap();

                    if f.starts_with("asd_") {
                        let data: AsdToken = serde_json::from_str(&raw).unwrap();
                        db.insert(
                            p.file_name().to_string_lossy().to_string(),
                            TokenType::AsdToken(data),
                        );
                    } else {
                        unimplemented!()
                    }
                }
            });
        }
        TokenStorage {
            path: path.into(),
            list: db,
        }
    }

    #[inline]
    pub fn store(&mut self, key: &str, data: TokenType) -> Result<()> {
        self.list.insert(key.into(), data);
        Ok(())
    }

    pub fn load(&self, key: &str) -> Result<TokenType> {
        match self.list.get(key) {
            Some(t) => Ok(t.clone()),
            None => Err(TokenStatus::NotFound(key.to_string()).into()),
        }
    }

    #[inline]
    pub fn path(&self) -> String {
        self.path.clone()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// List tokens
    ///
    /// NOTE: we do not show data from each token (like expiration, etc.) because at this point
    ///       we do not know which kind of token each one is.
    ///
    #[tracing::instrument(skip(self))]
    pub fn list(&self) -> Result<String> {
        trace!("listing tokens");

        let header = vec!["Path", "Producer", "Created at"];

        let mut builder = Builder::default();
        builder.push_record(header);

        let p = self.path.as_str();
        if let Ok(dir) = fs::read_dir(p) {
            for fname in dir {
                let mut row = vec![];

                if let Ok(fname) = fname {
                    // Using strings is easier
                    //
                    let name = format!("{}", fname.file_name().to_string_lossy());
                    row.push(name.clone());

                    // FIXME
                    if name.starts_with("asd_default_token") {
                        row.push("Asd".into());
                    } else {
                        row.push("Unknown".into());
                    }

                    let st = fname.metadata().unwrap();
                    let modified = DateTime::<Utc>::from(st.modified().unwrap());
                    let modified = format!("{}", modified);
                    row.push(modified);
                } else {
                    row.push("INVALID".to_string());
                    let origin = format!("{}", DateTime::<Utc>::from(UNIX_EPOCH));
                    row.push(origin);
                }
                builder.push_record(row);
            }
        }
        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all tokens:\n{}", table);
        Ok(table)
    }
}
