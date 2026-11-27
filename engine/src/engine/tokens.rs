//! Engine methods for accessing the shared token actor.

use eyre::Result;
use ractor::call;

use crate::{TokenMsg, TokenType};

use super::Engine;

impl Engine {
    /// Returns a formatted list of all currently available authentication tokens
    ///
    /// Provides information about active authentication tokens used for
    /// accessing various data sources.
    ///
    #[tracing::instrument(skip(self))]
    pub async fn list_tokens(&self) -> Result<String> {
        call!(self.tokens, TokenMsg::Table)?
    }

    /// Loads a token by key through the token actor.
    ///
    /// Returns a storage error if the key is unsupported, missing, or unreadable.
    pub async fn load_token(&self, key: &str) -> Result<TokenType> {
        call!(self.tokens, TokenMsg::Get, key.to_owned())?
    }

    /// Persists a token by key through the token actor.
    ///
    /// A successful reply means the storage backend accepted the write and the
    /// token actor updated its in-memory list.
    ///
    /// ```no_run
    /// # async fn example(engine: &fetiche_engine::Engine) -> eyre::Result<()> {
    /// use fetiche_engine::{TokenType, token::AsdToken};
    ///
    /// let key = "asd_example";
    /// engine.store_token(key, TokenType::AsdToken(AsdToken::default())).await?;
    /// let token = engine.load_token(key).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn store_token(&self, key: &str, token: TokenType) -> Result<()> {
        call!(self.tokens, TokenMsg::Store, key.to_owned(), token)?
    }
}
