//! Persistent token storage and its engine actor.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use eyre::Result;
use futures::TryStreamExt;
use jiff::Timestamp;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::{ObjectMeta, ObjectStore, ObjectStoreExt};
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, pg};
use tabled::builder::Builder;
use tabled::settings::Style;
use tokio::runtime::{Builder as RuntimeBuilder, Handle};
use tracing::{debug, info, trace};

use crate::token::AsdToken;
use crate::{ENGINE_PG, TokenStatus, TokenType};

/// Actor implementation for managing token storage and operations
///
/// Handles token retrieval, listing and persistence operations through
/// an async message-based interface.
#[derive(Clone, Debug)]
pub struct TokenActor;

/// Messages that can be sent to the TokenActor
#[derive(Debug)]
pub enum TokenMsg {
    /// Retrieve a token by its key
    Get(String, RpcReplyPort<Result<TokenType>>),
    /// List all available tokens
    List(RpcReplyPort<Vec<TokenType>>),
    /// Persist a typed token by key and acknowledge the result.
    Store(String, TokenType, RpcReplyPort<Result<()>>),
    /// Return the formatted token table.
    Table(RpcReplyPort<Result<String>>),
    /// Return the number of registered tokens.
    Count(RpcReplyPort<usize>),
    /// Return the registered storage path.
    Path(RpcReplyPort<String>),
}

#[ractor::async_trait]
impl Actor for TokenActor {
    type Msg = TokenMsg;
    type State = TokenStorage;
    type Arguments = PathBuf;

    /// Opens the token directory and joins the engine process group.
    #[tracing::instrument(skip(self, myself))]
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let tokens = TokenStorage::register(&args).await?;
        pg::join(ENGINE_PG.into(), vec![myself.get_cell()]);
        info!("{} tokens loaded", tokens.len());
        Ok(tokens)
    }

    /// Replies to requests while keeping caller and storage errors local to the request.
    #[tracing::instrument(skip(self, _myself, message, state))]
    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            TokenMsg::Get(key, reply) => {
                let _ = reply.send(state.load(&key).await);
            }
            TokenMsg::List(reply) => {
                let _ = reply.send(state.list());
            }
            TokenMsg::Store(key, token, reply) => {
                let _ = reply.send(state.store(&key, token).await);
            }
            TokenMsg::Table(reply) => {
                let _ = reply.send(state.as_string().await);
            }
            TokenMsg::Count(reply) => {
                let _ = reply.send(state.len());
            }
            TokenMsg::Path(reply) => {
                let _ = reply.send(state.path());
            }
        }
        Ok(())
    }
}

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
/// - `store`: An `Arc<LocalFileSystem>` providing the storage backend.
/// - `base_path`: A `Path` representing the base path in the object store for token files.
/// - `list`: A `BTreeMap` storing the tokens. The keys are file names, and the values are the parsed `TokenType`s.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use fetiche_engine::{TokenStorage, TokenType};
/// use fetiche_engine::token::AsdToken;
///
/// # #[tokio::main]
/// # async fn main() -> eyre::Result<()> {
/// let directory = tempfile::tempdir()?;
/// let path = PathBuf::from(directory.path());
/// // Register a directory containing token files
/// let mut storage = TokenStorage::register(&path).await?;
///
/// // Store a new token
/// let token_data = TokenType::AsdToken(AsdToken::default());
/// storage.store("asd_token_file", token_data).await?;
///
/// // Retrieve a token by key
/// if let Ok(token) = storage.load("asd_token_file").await {
///     println!("Loaded token: {:?}", token);
/// }
///
/// // List all tokens
/// if let Ok(token_list) = storage.as_string().await {
///     println!("{}", token_list);
/// }
/// # Ok(())
/// # }
/// ```
///
#[derive(Debug)]
pub struct TokenStorage {
    /// Object store backend
    store: Arc<LocalFileSystem>,
    /// Base path for token files
    base_path: Path,
    /// Btree of (key, AuthToken)
    list: BTreeMap<String, TokenType>,
}

impl TokenStorage {
    /// Reads the directory and loads its supported token files.
    ///
    /// # Errors
    ///
    /// Returns [`TokenStatus::Unknown`] for unsupported filenames,
    /// [`TokenStatus::Malformed`] for malformed token data, and preserves
    /// backend errors when an object cannot be read.
    ///
    #[tracing::instrument]
    pub async fn register(path: &PathBuf) -> Result<Self> {
        debug!("Registering token storage at {:?}", path);
        let store = LocalFileSystem::new_with_prefix(path)?;
        let mut db = BTreeMap::<String, TokenType>::new();

        // List all objects in the directory
        //
        let list_stream = store.list(None);
        let objects: Vec<ObjectMeta> = list_stream.try_collect().await?;

        trace!("reading directory {path:?}");

        for object in objects {
            if let Some(file_name) = object.location.filename() {
                let token = read_token(&store, &object.location, file_name).await?;
                db.insert(file_name.to_owned(), token);
            }
        }

        Ok(TokenStorage {
            store: Arc::new(store),
            base_path: Path::from_filesystem_path(path)?,
            list: db,
        })
    }

    /// Stores a token under a supported key and updates the in-memory list
    /// after the write succeeds. Unsupported keys return [`TokenStatus::Unknown`].
    #[tracing::instrument(skip(self, key, data))]
    pub async fn store(&mut self, key: &str, data: TokenType) -> Result<()> {
        validate_token_key(key)?;
        // Serialize the token data
        let serialized = match &data {
            TokenType::AsdToken(token) => serde_json::to_string(token)?,
        };

        // Create the full path for the token file
        let file_path = self.base_path.clone().join(key);

        // Store in the object store
        self.store.put(&file_path, serialized.into()).await?;

        // Update the in-memory cache
        self.list.insert(key.into(), data);

        Ok(())
    }

    /// Loads a token from the cache or falls back to storage.
    ///
    /// Unsupported keys, missing objects, malformed data, and other backend
    /// read errors have distinct [`TokenStatus`] variants.
    #[tracing::instrument(skip(self, key))]
    pub async fn load(&self, key: &str) -> Result<TokenType> {
        // First check the in-memory cache
        //
        if let Some(token) = self.list.get(key) {
            return Ok(token.clone());
        }

        // If not in cache, try to load from object store
        //
        let file_path = self.base_path.clone().join(key);

        read_token(&self.store, &file_path, key).await
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
        let list_stream = self.store.list(None);
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

                row.push(object.last_modified.to_string());
            } else {
                row.push("INVALID".to_string());
                row.push("Unknown".to_string());
                row.push(Timestamp::from_second(0).unwrap().to_string());
            }
            builder.push_record(row);
        }

        let table = builder.build().with(Style::rounded()).to_string();
        let table = format!("Listing all tokens:\n{}", table);
        Ok(table)
    }

    /// Loads the token directory from synchronous code.
    ///
    /// Returns the same validation errors as [`Self::register`]. Use the
    /// asynchronous API when calling from a Tokio runtime.
    ///
    /// ```
    /// # fn main() -> eyre::Result<()> {
    /// let dir = tempfile::tempdir()?;
    /// let storage = fetiche_engine::TokenStorage::register_sync(&dir.path().to_path_buf())?;
    /// assert!(storage.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument]
    pub fn register_sync(path: &PathBuf) -> Result<Self> {
        run_sync(Self::register(path))
    }

    /// Stores a token from synchronous code. Use [`Self::store`] in Tokio.
    #[tracing::instrument(skip(self, key, data))]
    pub fn store_sync(&mut self, key: &str, data: TokenType) -> Result<()> {
        run_sync(self.store(key, data))
    }

    /// Loads a token from synchronous code. Use [`Self::load`] in Tokio.
    #[tracing::instrument(skip(self, key))]
    pub fn load_sync(&self, key: &str) -> Result<TokenType> {
        run_sync(self.load(key))
    }

    /// Lists tokens from synchronous code. Use [`Self::as_string`] in Tokio.
    #[tracing::instrument(skip(self))]
    pub fn to_string_sync(&self) -> Result<String> {
        run_sync(self.as_string())
    }
}

/// Checks that the key names a format understood by this store.
fn validate_token_key(key: &str) -> Result<(), TokenStatus> {
    if !key.starts_with("asd_") {
        return Err(TokenStatus::Unknown(key.to_owned()));
    }
    Ok(())
}

/// Reads a supported token object and reports malformed JSON without quoting its contents.
async fn read_token(store: &LocalFileSystem, location: &Path, key: &str) -> Result<TokenType> {
    validate_token_key(key)?;
    let data = store
        .get(location)
        .await
        .map_err(|err| token_read_error(key, err))?;
    let bytes = data
        .bytes()
        .await
        .map_err(|err| token_read_error(key, err))?;
    let token: AsdToken = serde_json::from_slice(&bytes).map_err(|err| TokenStatus::Malformed {
        key: key.to_owned(),
        line: err.line(),
        column: err.column(),
    })?;
    Ok(TokenType::AsdToken(token))
}

/// Distinguishes a missing object from other storage failures.
fn token_read_error(key: &str, source: object_store::Error) -> TokenStatus {
    if matches!(source, object_store::Error::NotFound { .. }) {
        TokenStatus::NotFound(key.to_owned())
    } else {
        TokenStatus::Read {
            key: key.to_owned(),
            source,
        }
    }
}

/// Runs an async token operation on a temporary runtime from a synchronous caller.
fn run_sync<T>(operation: impl Future<Output = Result<T>>) -> Result<T> {
    if Handle::try_current().is_ok() {
        return Err(TokenStatus::BlockingCallInRuntime.into());
    }
    RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::path::PathBuf;
    use tempfile::{TempDir, tempdir};

    #[fixture]
    fn temp_dir() -> TempDir {
        tempdir().unwrap()
    }

    #[rstest]
    #[tokio::test]
    async fn test_register_empty_directory(temp_dir: TempDir) -> Result<()> {
        let temp = PathBuf::from(temp_dir.path());
        let storage = TokenStorage::register(&temp).await?;
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
        Ok(())
    }

    #[rstest]
    #[tokio::test]
    async fn test_store_and_load(temp_dir: TempDir) -> Result<()> {
        let temp = PathBuf::from(temp_dir.path());
        let mut storage = TokenStorage::register(&temp).await?;

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
    async fn test_load_nonexistent(temp_dir: TempDir) -> Result<()> {
        let temp = PathBuf::from(temp_dir.path());
        let storage = TokenStorage::register(&temp).await?;
        let err = storage.load("asd_nonexistent").await.unwrap_err();
        assert!(matches!(
            err.downcast_ref::<TokenStatus>(),
            Some(TokenStatus::NotFound(key)) if key == "asd_nonexistent"
        ));
        Ok(())
    }

    /// Unsupported filenames produce a typed error in both token APIs.
    #[test]
    fn test_register_sync_reports_unsupported_token_file() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().to_path_buf();
        assert!(TokenStorage::register_sync(&path)?.is_empty());
        std::fs::write(path.join("unknown.json"), "not a token")?;
        let err = TokenStorage::register_sync(&path).unwrap_err();
        assert!(matches!(
            err.downcast_ref::<TokenStatus>(),
            Some(TokenStatus::Unknown(key)) if key == "unknown.json"
        ));
        Ok(())
    }

    /// Blocking token methods reject calls from Tokio instead of panicking.
    #[tokio::test]
    async fn test_sync_calls_return_error_inside_tokio() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().to_path_buf();
        let mut storage = TokenStorage::register(&path).await?;
        for err in [
            TokenStorage::register_sync(&path).unwrap_err(),
            storage
                .store_sync("asd_token", TokenType::AsdToken(AsdToken::default()))
                .unwrap_err(),
            storage.load_sync("asd_token").unwrap_err(),
            storage.to_string_sync().unwrap_err(),
        ] {
            assert!(matches!(
                err.downcast_ref::<TokenStatus>(),
                Some(TokenStatus::BlockingCallInRuntime)
            ));
        }
        Ok(())
    }

    /// A malformed on-disk record reports its location without echoing credential values.
    #[tokio::test]
    async fn test_malformed_token_is_reported_without_its_contents() -> Result<()> {
        let dir = tempdir()?;
        let path = dir.path().to_path_buf();
        let storage = TokenStorage::register(&path).await?;
        let key = "asd_corrupt";
        storage
            .store
            .put(
                &storage.base_path.clone().join(key),
                r#"{"expiredAt":"fake-secret"}"#.into(),
            )
            .await?;
        for err in [
            storage.load(key).await.unwrap_err(),
            TokenStorage::register(&path).await.unwrap_err(),
        ] {
            assert!(matches!(
                err.downcast_ref::<TokenStatus>(),
                Some(TokenStatus::Malformed { key: name, line: 1, .. }) if name == key
            ));
            assert!(!format!("{err:#}").contains("fake-secret"));
        }
        Ok(())
    }

    /// Unsupported keys are rejected for both reading and writing.
    #[tokio::test]
    async fn test_unsupported_key_cannot_be_loaded_or_stored() -> Result<()> {
        let dir = tempdir()?;
        let mut storage = TokenStorage::register(&dir.path().to_path_buf()).await?;
        for err in [
            storage.load("unsupported").await.unwrap_err(),
            storage
                .store("unsupported", TokenType::AsdToken(AsdToken::default()))
                .await
                .unwrap_err(),
        ] {
            assert!(matches!(
                err.downcast_ref::<TokenStatus>(),
                Some(TokenStatus::Unknown(_))
            ));
        }
        assert!(storage.is_empty());
        Ok(())
    }

    /// Non-missing backend failures retain their original cause.
    #[test]
    fn test_token_read_error_preserves_io_failure() {
        let backend = object_store::Error::Generic {
            store: "test",
            source: Box::new(std::io::Error::from(std::io::ErrorKind::PermissionDenied)),
        };
        let mapped = token_read_error("asd_token", backend);
        assert!(matches!(
            std::error::Error::source(&mapped).and_then(|source| source.source())
                .and_then(|cause| cause.downcast_ref::<std::io::Error>()),
            Some(cause) if cause.kind() == std::io::ErrorKind::PermissionDenied
        ));
    }

    #[rstest]
    #[tokio::test]
    async fn test_as_string(temp_dir: TempDir) -> Result<()> {
        let temp = PathBuf::from(temp_dir.path());
        let mut storage = TokenStorage::register(&temp).await?;

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
    async fn test_list(temp_dir: TempDir) -> Result<()> {
        let temp = PathBuf::from(temp_dir.path());
        let mut storage = TokenStorage::register(&temp).await?;

        let token = AsdToken::default();
        let token_type = TokenType::AsdToken(token);
        storage.store("asd_test_token", token_type.clone()).await?;

        let tokens = storage.list();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], token_type);
        Ok(())
    }
}
