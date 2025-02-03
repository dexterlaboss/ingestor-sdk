
use {
    crate::{
        uploader_config::UploaderConfig,
        cache_config::LedgerCacheConfig,
    },
    std::{
        time::{Duration},
    },
};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:9090";

#[derive(Debug)]
pub struct LedgerStorageConfig {
    pub read_only: bool,
    pub timeout: Option<Duration>,
    pub address: String,
    pub uploader_config: UploaderConfig,
    pub cache_config: LedgerCacheConfig,
}

impl Default for LedgerStorageConfig {
    fn default() -> Self {
        Self {
            read_only: false,
            timeout: None,
            address: DEFAULT_ADDRESS.to_string(),
            uploader_config: UploaderConfig::default(),
            cache_config: LedgerCacheConfig::default(),
        }
    }
}