
use {
    std::{
        time::{Duration},
    },
};

pub const DEFAULT_MEMCACHE_ADDRESS: &str = "127.0.0.1:11211";

#[derive(Debug, Clone)]
pub struct LedgerCacheConfig {
    pub enable_full_tx_cache: bool,
    pub address: String,
    pub timeout: Option<Duration>,
    pub tx_cache_expiration: Option<Duration>,
}

impl Default for LedgerCacheConfig {
    fn default() -> Self {
        Self {
            enable_full_tx_cache: false,
            address: DEFAULT_MEMCACHE_ADDRESS.to_string(),
            timeout: Some(Duration::from_secs(1)),
            tx_cache_expiration: Some(Duration::from_secs(60 * 60 * 24 * 14)), // 14 days
        }
    }
}