use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use eyre::Result;
use futures::TryStreamExt;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore};
use tabled::builder::Builder;
use tabled::settings::Style;
use tokio::runtime::Handle;
use tracing::trace;

use crate::token::AsdToken;
use crate::{TokenStatus, TokenType};

/// The `TokenStorage` struct provides functionality for managing tokens
/// stored as serialized files using the `object_store` crate. It allows operations
/// such as registering the directory containing token files, storing tokens,
/// and retrieving tokens by key, as well as listing all tokens present in the
/// directory.
///
/// This struct is specifically designed to work with different types of tokens,
/// which are represented by the `TokenType` enum. The specific token format is
/// determined based on the file content.
///
/// # Fields
/// - `store`: An `Arc<dyn ObjectStore>` providing the storage backend.
/// - `base_path`: A `Path` representing the base path in the object store for token files.
/// - `list`: A `BTreeMap` storing the tokens. The keys are file names, and the values are the parsed `TokenType`s.
///
/// # Examples
///
/// ```no_run
/// use futures::executor::block_on;
/// use fetiche_engine::{TokenStorage, TokenType};
/// use fetiche_engine::token::AsdToken;
///
/// # block_on(async {
/// // Register a directory containing token files
/// let mut storage = TokenStorage::register("path/to/tokens").await?;
///
/// // Store a new token
/// let token_data = TokenType::AsdToken(AsdToken::default());
/// let _ = storage.store("asd_token_file", token_data).await;
///
/// // Retrieve a token by key
/// if let Ok(token) = storage.load("asd_existing_token_file").await {
///     println!("Loaded token: {:?}", token);
/// }
///
/// // List all tokens
/// if let Ok(token_list) = storage.as_string().await {
///     println!("{}", token_list);
/// }
/// # Ok::<(), eyre::Report>(())
/// # })?;
/// ```
///
#[derive(Debug)]
pub struct TokenStorage {
    /// Object store backend
    store: Arc<dyn ObjectStore>,
    /// Base path for token files
    base_path: Path,
    /// Btree of (key, AuthToken)
    list: BTreeMap<String, TokenType>,
}

impl TokenStorage {
    /// Read the directory and return all tokens (one per file)
    ///
    #[tracing::instrument]
    pub async fn register(path: &str) -> Result<Self> {
        let store = Arc::new(LocalFileSystem::new());
        let base_path = Path::from(path);
        let mut db = BTreeMap::<String, TokenType>::new();

        // List all objects in the directory
        let list_stream = store.list(Some(&base_path));
        let objects: Vec<ObjectMeta> = list_stream.try_collect().await?;

        trace!("reading directory {path}");

        for object in objects {
            if let Some(file_name) = object.location.filename() {
                trace!("Processing file: {}", file_name);

                let data = store.get(&object.location).await?;
                let bytes = data.bytes().await?;
                let raw = String::from_utf8(bytes.to_vec())?;

                if file_name.starts_with("asd_") {
                    let token_data: AsdToken = serde_json::from_str(&raw)?;
                    db.insert(file_name.to_string(), TokenType::AsdToken(token_data));
                } else {
                    unimplemented!("Unsupported token type for file: {}", file_name)
                }
            }
        }

        Ok(TokenStorage {
            store,
            base_path,
            list: db,
        })
    }

    /// Store a token in the object store and update the in-memory list
    #[tracing::instrument(skip(self))]
    pub async fn store(&mut self, key: &str, data: TokenType) -> Result<()> {
        // Serialize the token data
        let serialized = match &data {
            TokenType::AsdToken(token) => serde_json::to_string(token)?,
        };

        // Create the full path for the token file
        let file_path = self.base_path.child(key);

        // Store in the object store
        self.store.put(&file_path, serialized.into()).await?;

        // Update the in-memory cache
        self.list.insert(key.into(), data);

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn load(&self, key: &str) -> Result<TokenType> {
        // First check the in-memory cache
        //
        if let Some(token) = self.list.get(key) {
            return Ok(token.clone());
        }

        // If not in cache, try to load from object store
        //
        let file_path = self.base_path.child(key);

        match self.store.get(&file_path).await {
            Ok(data) => {
                let bytes = data.bytes().await?;
                let raw = String::from_utf8(bytes.to_vec())?;

                if key.starts_with("asd_") {
                    let token_data: AsdToken = serde_json::from_str(&raw)?;
                    Ok(TokenType::AsdToken(token_data))
                } else {
                    unimplemented!("Unsupported token type for key: {}", key)
                }
            }
            Err(_) => Err(TokenStatus::NotFound(key.to_string()).into()),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn path(&self) -> String {
        self.base_path.to_string()
    }

    #[tracing::instrument(skip(self))]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    #[tracing::instrument(skip(self))]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    #[tracing::instrument(skip(self))]
    pub fn list(&self) -> Vec<TokenType> {
        self.list.values().cloned().collect()
    }

    /// List tokens as a string
    ///
    /// NOTE: we do not show data from each token (like expiration, etc.) because at this point
    ///       we do not know which kind of token each one is.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn as_string(&self) -> Result<String> {
        trace!("listing tokens");

        let header = vec!["Path", "Producer", "Created at"];

        let mut builder = Builder::default();
        builder.push_record(header);

        // List all objects in the base path
        let list_stream = self.store.list(Some(&self.base_path));
        let objects: Vec<ObjectMeta> = list_stream.try_collect().await?;

        for object in objects {
            let mut row = vec![];

            if let Some(file_name) = object.location.filename() {
                row.push(file_name.to_string());

                // FIXME: Determine producer based on file name
                if file_name.starts_with("asd_default_token") {
                    row.push("Asd".into());
                } else {
                    row.push("Unknown".into());
                }

                let modified = DateTime::<Utc>::from(object.last_modified);
                let modified = format!("{}", modified);
                row.push(modified);
            } else {
                row.push("INVALID".to_string());
                row.push("Unknown".to_string());
                let origin = format!("{}", DateTime::<Utc>::from(std::time::UNIX_EPOCH));
                row.push(origin);
            }
            builder.push_record(row);
        }

        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all tokens:\n{}", table);
        Ok(table)
    }

    /// Synchronous version of register for backward compatibility
    #[tracing::instrument]
    pub fn register_sync(path: &str) -> Self {
        let rt = Handle::current();
        rt.block_on(async { Self::register(path).await.unwrap() })
    }

    /// Synchronous version of store for backward compatibility
    #[tracing::instrument(skip(self))]
    pub fn store_sync(&mut self, key: &str, data: TokenType) -> Result<()> {
        let rt = Handle::current();
        rt.block_on(async { self.store(key, data).await })
    }

    /// Synchronous version of load for backward compatibility
    #[tracing::instrument(skip(self))]
    pub fn load_sync(&self, key: &str) -> Result<TokenType> {
        let rt = Handle::current();
        rt.block_on(async { self.load(key).await })
    }

    /// Synchronous version of to_string for backward compatibility
    #[tracing::instrument(skip(self))]
    pub fn to_string_sync(&self) -> Result<String> {
        let rt = Handle::current();
        rt.block_on(async { self.as_string().await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use tempfile::tempdir;

    #[fixture]
    fn temp_dir() -> String {
        let dir = tempdir().unwrap();
        dir.path().to_str().unwrap().to_string()
    }

    #[rstest]
    #[tokio::test]
    async fn test_register_empty_directory(temp_dir: String) -> Result<()> {
        let storage = TokenStorage::register(&temp_dir).await?;
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_store_and_load(temp_dir: String) -> Result<()> {
        let mut storage = TokenStorage::register(&temp_dir).await?;

        let token = AsdToken::default();
        let token_type = TokenType::AsdToken(token.clone());
        storage.store("asd_test_token", token_type.clone()).await?;

        assert_eq!(storage.len(), 1);
        assert!(!storage.is_empty());

        let loaded = storage.load("asd_test_token").await?;
        assert_eq!(loaded, token_type);
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_load_nonexistent(temp_dir: String) -> Result<()> {
        let storage = TokenStorage::register(&temp_dir).await?;
        let result = storage.load("nonexistent").await;
        assert!(result.is_err());
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_as_string(temp_dir: String) -> Result<()> {
        let mut storage = TokenStorage::register(&temp_dir).await?;

        let token = AsdToken::default();
        let token_type = TokenType::AsdToken(token);
        storage.store("asd_default_token", token_type).await?;

        let output = storage.as_string().await?;
        assert!(output.contains("asd_default_token"));
        assert!(output.contains("Asd"));
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_list(temp_dir: String) -> Result<()> {
        let mut storage = TokenStorage::register(&temp_dir).await?;

        let token = AsdToken::default();
        let token_type = TokenType::AsdToken(token);
        storage.store("asd_test_token", token_type.clone()).await?;

        let tokens = storage.list();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], token_type);
        Ok(())
    }
}
