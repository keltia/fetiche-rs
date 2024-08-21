use chrono::Utc;
use enum_dispatch::enum_dispatch;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs;
use std::fs::read_dir;
use std::path::Path;

#[enum_dispatch(TokenType)]
pub trait Expirable: Debug + Clone {
    fn key(&self) -> String;
    fn is_expired(&self) -> bool;
}

#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenType {
    AsdToken,
}

#[derive(Debug)]
pub struct TokenStorage {
    /// `path` is relative to `root`.
    path: String,
    /// Btree of (key, AuthToken)
    list: BTreeMap<String, TokenType>,
}

impl TokenStorage {
    pub fn register(path: &str) -> Self {
        let mut db = BTreeMap::<String, TokenType>::new();
        if let Ok(dir) = read_dir(&path) {
            dir.into_iter().for_each(|entry| {
                if let Ok(p) = entry {
                    let f = p.file_name().to_str().unwrap().to_string();
                    let full = Path::new(path).join(f.as_str());
                    let raw = fs::read_to_string(full).unwrap();

                    if f.starts_with("asd_") {
                        let data: AsdToken = serde_json::from_str(&raw).unwrap();
                        db.insert(p.file_name().to_string_lossy().to_string(), data.into());
                    } else {
                        unimplemented!()
                    }
                }
            });
        }
        TokenStorage { path: path.into(), list: db }
    }

    pub fn store(&mut self, key: &str, data: TokenType) -> Result<()> {
        self.list.insert(key.into(), data);
        Ok(())
    }

    pub fn load(&self, key: &str) -> Result<TokenType> {
        match self.list.get(key) {
            Some(t) => Ok(t.clone()),
            None => Err(eyre!("Unknown token").into())
        }
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// Access token derived from username/password
///
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AsdToken {
    /// The actual token
    token: String,
    /// Don't ask
    gjrt: String,
    /// Expiration date
    expired_at: i64,
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

impl AsdToken {
    pub fn export(&self) -> Result<String> {
        Ok(json!(&self).to_string())
    }
}

impl Expirable for AsdToken {
    fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expired_at
    }

    fn key(&self) -> String {
        self.email.clone()
    }
}
