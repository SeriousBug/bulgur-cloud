use std::fmt;

use lru_time_cache::LruCache;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use simple_secrecy;

#[cfg(feature = "generate_types")]
use typescript_type_def::TypeDef;
use utoipa::Component;

#[derive(
    Serialize,
    Deserialize,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Clone,
    simple_secrecy::Debug,
    simple_secrecy::Display,
)]
#[cfg_attr(feature = "generate_types", derive(TypeDef))]
#[derive(Component)]
#[component(example = "IEFbt3Nn7szVn9POmzfft")]
pub struct Token(String);

impl Token {
    pub fn read(s: &str) -> Token {
        Token(s.to_string())
    }

    pub fn new() -> Token {
        Token(nanoid!())
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "generate_types", derive(TypeDef))]
pub struct PathTokenResponse {
    pub token: Token,
}

#[derive(std::fmt::Debug, Serialize, Deserialize, derive_more::Display, Clone)]
pub struct User(pub String);

#[derive(simple_secrecy::Debug, simple_secrecy::Display)]
pub struct TokenCache(pub RwLock<LruCache<Token, User>>);

impl TokenCache {
    pub fn new(cache_secs: u64) -> TokenCache {
        TokenCache(RwLock::new(LruCache::with_expiry_duration(
            std::time::Duration::from_secs(cache_secs),
        )))
    }
}

#[derive(simple_secrecy::Debug, simple_secrecy::Display)]
pub struct PathTokenCache(pub RwLock<LruCache<String, Token>>);

impl PathTokenCache {
    pub fn new(cache_capacity: usize) -> PathTokenCache {
        PathTokenCache(RwLock::new(LruCache::with_capacity(cache_capacity)))
    }
}

pub struct AppState {
    pub started_at: chrono::DateTime<chrono::Local>,
    /// Maps tokens to usernames
    pub token_cache: TokenCache,
    /// Maps paths to tokens
    pub path_token_cache: PathTokenCache,
}

#[cfg(debug_assertions)]
fn debug_token_cache(cache: &TokenCache) -> String {
    cache.0.blocking_read().len().to_string()
}

#[cfg(not(debug_assertions))]
fn debug_token_cache(_cache: &TokenCache) -> String {
    "TokenCache".to_string()
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token_cache = debug_token_cache(&self.token_cache);
        f.debug_struct("AppState")
            .field("started_at", &self.started_at)
            .field("token_cache", &token_cache)
            .finish()
    }
}

#[derive(Clone, simple_secrecy::Debug, simple_secrecy::Display)]
pub enum Authorized {
    User(User),
    Path,
    Both(User),
}
