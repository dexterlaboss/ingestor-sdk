
use {
    std::{
        time::{Duration},
        num::NonZeroUsize,
    },
};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:9090";

#[derive(Debug)]
pub struct LedgerStorageConfig {
    pub read_only: bool,
    pub timeout: Option<Duration>,
    pub address: String,
    pub block_cache: Option<NonZeroUsize>,
    pub use_md5_row_key_salt: bool,
    pub enable_full_tx_cache: bool,
    pub disable_tx_fallback: bool,
    pub cache_address: Option<String>,
}

impl Default for LedgerStorageConfig {
    fn default() -> Self {
        Self {
            read_only: true,
            timeout: None,
            address: DEFAULT_ADDRESS.to_string(),
            block_cache: None,
            use_md5_row_key_salt: false,
            enable_full_tx_cache: false,
            disable_tx_fallback: false,
            cache_address: Some(DEFAULT_ADDRESS.to_string()),
        }
    }
}