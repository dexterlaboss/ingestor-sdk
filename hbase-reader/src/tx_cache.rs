
use {
    log::*,
    // solana_sdk::{
    //     signature::Signature,
    // },
    solana_signature::Signature,
    solana_storage_reader::{
        Error, Result,
    },
    solana_storage_utils::{
        compression::{decompress},
    },
    memcache::Client as MemcacheClient,
    tokio::task,
};


#[derive(Debug)]
pub struct CacheErrorWrapper(pub memcache::MemcacheError);

impl From<CacheErrorWrapper> for Error {
    fn from(err: CacheErrorWrapper) -> Self {
        Error::CacheError(err.0.to_string())  // Convert the wrapped error into a string and store it in CacheError
    }
}

pub(crate) async fn get_cached_transaction<P>(
    cache_client: &MemcacheClient,
    signature: &Signature,
) -> Result<Option<P>>
where
    P: prost::Message + Default
{
    let key = signature.to_string();
    let key_clone = key.clone();
    let cache_client_clone = cache_client.clone();

    let result = task::spawn_blocking(move || {
        cache_client_clone.get::<Vec<u8>>(&key_clone).map_err(CacheErrorWrapper)
    })
        .await
        .map_err(Error::TokioJoinError)??;

    if let Some(cached_bytes) = result {
        // Decompress the cached data
        let data = decompress(&cached_bytes).map_err(|e| {
            warn!("Failed to decompress transaction from cache for {}", key);
            Error::CacheError(format!("Decompression error: {}", e))
        })?;

        // Deserialize the data using protobuf instead of bincode
        let tx = P::decode(&data[..]).map_err(|e| {
            warn!("Failed to deserialize transaction from cache for {}", key);
            Error::CacheError(format!("Protobuf deserialization error: {}", e))
        })?;

        debug!("Transaction {} found in cache", key);
        return Ok(Some(tx));
    }

    Ok(None)
}