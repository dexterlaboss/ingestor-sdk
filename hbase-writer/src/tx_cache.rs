#![allow(clippy::integer_arithmetic)]

use {
    solana_storage_writer::{
        compression::{compress_best},
    },
    memcache::{Client, MemcacheError},
};

#[derive(Debug)]
pub enum CacheWriteError {
    MemcacheError(MemcacheError),         // Error from cache client
    IoError(std::io::Error),                // Error from encoding (e.g., Protobuf)
    EncodingError(prost::EncodeError),
}

pub async fn cache_transaction<T>(
    cache_client: &Client,
    signature: &str,
    transaction: T,
    tx_cache_expiration: Option<std::time::Duration>,
) -> Result<(), CacheWriteError>
where
    T: prost::Message,
{
    let mut buf = Vec::with_capacity(transaction.encoded_len());

    transaction.encode(&mut buf).map_err(CacheWriteError::EncodingError)?;

    let compressed_tx = compress_best(&buf).map_err(CacheWriteError::IoError)?;

    let expiration = tx_cache_expiration
        .map(|d| d.as_secs().min(u32::MAX as u64) as u32)
        .unwrap_or(0);

    cache_client
        .set(signature, compressed_tx.as_slice(), expiration)
        .map_err(CacheWriteError::MemcacheError)?;

    Ok(())
}