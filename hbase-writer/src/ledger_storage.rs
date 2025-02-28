#![allow(clippy::integer_arithmetic)]

use {
    crate::{
        hbase::Error as HBaseError,
        connection::HBaseConnection,
        tx_cache::cache_transaction,
        tx_utils::{
            get_account_keys,
            is_error_tx,
            is_voting_tx,
            is_program_account,
            convert_to_transaction_with_status_meta,
        },
        storage_config::LedgerStorageConfig,
        uploader_config::UploaderConfig,
        tx_cache::CacheWriteError,
    },
    async_trait::async_trait,
    log::*,
    // TODO: Implement metrics
    // solana_metrics::Metrics,
    //-------------------------
    // solana_metrics::{datapoint_info, inc_new_counter_debug},
    solana_sdk::{
        clock::{
            Slot,
        },
        pubkey::Pubkey,
        sysvar::is_sysvar_id,
    },
    dexter_storage_proto::convert::{generated, tx_by_addr},
    solana_transaction_status::{
        extract_memos::extract_and_fmt_memos,
        ConfirmedTransactionWithStatusMeta,
        TransactionByAddrInfo,
        VersionedConfirmedBlock,
        VersionedTransactionWithStatusMeta,
    },
    solana_storage_writer::{
        Error as StorageError,
        LedgerStorageAdapter,
    },
    solana_storage_utils::{
        tx_info::TransactionInfo,
        slot_to_blocks_key,
        slot_to_tx_by_addr_key,
    },
    std::{
        collections::{
            HashMap,
        },
        boxed::Box,
    },
    thiserror::Error,
    memcache::{Client, MemcacheError},
    tokio::{
        task::JoinError,
    },
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Storage Error: {0}")]
    StorageBackendError(Box<dyn std::error::Error + Send>),

    #[error("I/O Error: {0}")]
    IoError(std::io::Error),

    #[error("Transaction encoded is not supported")]
    UnsupportedTransactionEncoding,

    #[error("Block not found: {0}")]
    BlockNotFound(Slot),

    #[error("Signature not found")]
    SignatureNotFound,

    #[error("tokio error")]
    TokioJoinError(JoinError),

    #[error("Cache Error: {0}")]
    CacheError(Box<dyn std::error::Error + Send>),

    #[error("Protobuf error: {0}")]
    EncodingError(prost::EncodeError),
}

impl std::convert::From<HBaseError> for Error {
    fn from(err: HBaseError) -> Self {
        Self::StorageBackendError(Box::new(err))
    }
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<MemcacheError> for Error {
    fn from(err: MemcacheError) -> Self {
        Self::CacheError(Box::new(err))
    }
}

impl From<CacheWriteError> for TaskError {
    fn from(error: CacheWriteError) -> Self {
        match error {
            CacheWriteError::MemcacheError(e) => TaskError::MemcacheError(e),
            CacheWriteError::IoError(e) => TaskError::IoError(e),
            CacheWriteError::EncodingError(e) => TaskError::EncodingError(e),
        }
    }
}

impl From<TaskError> for Error {
    fn from(err: TaskError) -> Self {
        match err {
            TaskError::HBaseError(hbase_err) => Error::StorageBackendError(Box::new(hbase_err)),
            TaskError::MemcacheError(memcache_err) => Error::CacheError(Box::new(memcache_err)),
            TaskError::IoError(io_err) => Error::IoError(io_err),
            TaskError::EncodingError(enc_err) => Error::EncodingError(enc_err),
        }
    }
}

pub type Result<T> = std::result::Result<T, StorageError>;

enum TaskResult {
    BytesWritten(usize),
    CachedTransactions(usize),
}

#[derive(Debug)]
enum TaskError {
    HBaseError(HBaseError),
    MemcacheError(MemcacheError),
    IoError(std::io::Error),
    EncodingError(prost::EncodeError)
}

impl From<std::io::Error> for TaskError {
    fn from(err: std::io::Error) -> Self {
        TaskError::IoError(err)
    }
}

impl From<HBaseError> for TaskError {
    fn from(err: HBaseError) -> Self {
        TaskError::HBaseError(err)
    }
}

impl From<MemcacheError> for TaskError {
    fn from(err: MemcacheError) -> Self {
        TaskError::MemcacheError(err)
    }
}


#[derive(Clone)]
pub struct LedgerStorage {
    connection: HBaseConnection,
    uploader_config: UploaderConfig,
    cache_client: Option<Client>,
    enable_full_tx_cache: bool,
    tx_cache_expiration: Option<std::time::Duration>,
}

impl LedgerStorage {
    pub async fn new(
        read_only: bool,
        timeout: Option<std::time::Duration>,
    ) -> Self {
        Self::new_with_config(LedgerStorageConfig {
            read_only,
            timeout,
            ..LedgerStorageConfig::default()
        })
            .await
    }

    pub async fn new_with_config(config: LedgerStorageConfig) -> Self {
        let LedgerStorageConfig {
            read_only,
            timeout,
            address,
            uploader_config,
            cache_config,
        } = config;
        let connection = HBaseConnection::new(
            address.as_str(),
            read_only,
            timeout,
        )
            .await;

        let cache_client = if cache_config.enable_full_tx_cache {
            // Add the "memcache://" prefix programmatically
            let memcache_url = format!("memcache://{}?protocol=ascii", cache_config.address);
            Some(Client::connect(memcache_url.as_str()).unwrap())
        } else {
            None
        };

        Self {
            connection,
            uploader_config,
            cache_client,
            enable_full_tx_cache: cache_config.enable_full_tx_cache,
            tx_cache_expiration: cache_config.tx_cache_expiration,
        }
    }
}

#[async_trait]
impl LedgerStorageAdapter for LedgerStorage {
    async fn upload_confirmed_block(
        &self,
        slot: Slot,
        confirmed_block: VersionedConfirmedBlock,
    ) -> Result<()> {
        let mut by_addr: HashMap<&Pubkey, Vec<TransactionByAddrInfo>> = HashMap::new();

        info!("HBase: Uploading block {:?} from slot {:?}", confirmed_block.blockhash, slot);

        let mut tx_cells = vec![];
        let mut full_tx_cells = vec![];
        let mut full_tx_cache = vec![];
        for (index, transaction_with_meta) in confirmed_block.transactions.iter().enumerate() {
            let VersionedTransactionWithStatusMeta { meta, transaction } = transaction_with_meta;
            let err = meta.status.clone().err();
            let index = index as u32;
            let signature = transaction.signatures[0];
            let memo = extract_and_fmt_memos(transaction_with_meta);

            // let mut should_skip_tx = false;
            let mut should_skip_tx_by_addr = false;
            let mut should_skip_full_tx = false;

            let is_voting = is_voting_tx(transaction_with_meta);

            if self.uploader_config.filter_voting_tx && is_voting {
                should_skip_tx_by_addr = true;
                should_skip_full_tx = true;
            }

            let is_error = is_error_tx(transaction_with_meta);

            if self.uploader_config.filter_error_tx && is_error {
                should_skip_full_tx = true;
            }

            let combined_keys = get_account_keys(&transaction_with_meta);

            if !should_skip_tx_by_addr {
                for address in transaction_with_meta.account_keys().iter() {
                    // Filter program accounts from tx-by-addr index
                    if self.uploader_config.filter_program_accounts
                        && is_program_account(address, transaction_with_meta, &combined_keys) {
                        continue;
                    }

                    if should_skip_full_tx || !self.should_include_in_tx_full(address) {
                        should_skip_full_tx = true;
                    }

                    if !is_sysvar_id(address) && self.should_include_in_tx_by_addr(address) {
                        by_addr
                            .entry(address)
                            .or_default()
                            .push(TransactionByAddrInfo {
                                signature,
                                err: err.clone(),
                                index,
                                memo: memo.clone(),
                                block_time: confirmed_block.block_time,
                            });
                    }
                }
            }

            if self.uploader_config.enable_full_tx && !should_skip_full_tx {
                // should_skip_tx = true;

                full_tx_cells.push((
                    signature.to_string(),
                    ConfirmedTransactionWithStatusMeta {
                        slot,
                        tx_with_meta: convert_to_transaction_with_status_meta(transaction_with_meta.clone()),
                        block_time: confirmed_block.block_time,
                    }.into()
                ));
            }

            if self.enable_full_tx_cache
                && !is_voting
                && !transaction_with_meta.meta.status.is_err() {
                full_tx_cache.push((
                    signature.to_string(),
                    ConfirmedTransactionWithStatusMeta {
                        slot,
                        tx_with_meta: convert_to_transaction_with_status_meta(transaction_with_meta.clone()),
                        block_time: confirmed_block.block_time,
                    }
                ));
            }

            if !self.uploader_config.disable_tx /*&& !should_skip_tx*/ {
                tx_cells.push((
                    signature.to_string(),
                    TransactionInfo {
                        slot,
                        index,
                        err,
                        // memo,
                    },
                ));
            }
        }

        let tx_by_addr_cells: Vec<_> = by_addr
            .into_iter()
            .map(|(address, transaction_info_by_addr)| {
                (
                    format!("{}/{}", address, slot_to_tx_by_addr_key(slot)),
                    tx_by_addr::TransactionByAddr {
                        tx_by_addrs: transaction_info_by_addr
                            .into_iter()
                            .map(|by_addr| by_addr.into())
                            .collect(),
                    },
                )
            })
            .collect();

        let mut tasks = vec![];

        if !full_tx_cells.is_empty() && self.uploader_config.enable_full_tx {
            let conn = self.connection.clone();
            let full_tx_table_name = self.uploader_config.full_tx_table_name.clone();
            let use_tx_full_compression = self.uploader_config.use_tx_full_compression.clone();
            let write_to_wal = self.uploader_config.hbase_write_to_wal.clone();
            tasks.push(tokio::spawn(async move {
                conn.put_protobuf_cells_with_retry::<generated::ConfirmedTransactionWithStatusMeta>(
                    full_tx_table_name.as_str(),
                    &full_tx_cells,
                    use_tx_full_compression,
                    write_to_wal,
                )
                    .await
                    .map(TaskResult::BytesWritten)
                    .map_err(TaskError::from)
            }));
        }

        if !full_tx_cache.is_empty() && self.enable_full_tx_cache {
            let mut cached_count = 0;
            let cache_client = self.cache_client.clone();
            let tx_cache_expiration = self.tx_cache_expiration;
            debug!("Writing block transactions to cache");
            tasks.push(tokio::spawn(async move {
                for (signature, transaction) in full_tx_cache {
                    if let Some(client) = &cache_client {
                        cache_transaction::<generated::ConfirmedTransactionWithStatusMeta>(
                            &client,
                            &signature,
                            transaction.into(),
                            tx_cache_expiration,
                        )
                            .await
                            .map_err(TaskError::from)?;

                        cached_count += 1;
                        debug!("Cached transaction with signature {}", signature);
                    }
                }
                Ok::<TaskResult, TaskError>(TaskResult::CachedTransactions(cached_count))
            }));
        }

        if !tx_cells.is_empty() && !self.uploader_config.disable_tx {
            let conn = self.connection.clone();
            let tx_table_name = self.uploader_config.tx_table_name.clone();
            let use_tx_compression = self.uploader_config.use_tx_compression.clone();
            let write_to_wal = self.uploader_config.hbase_write_to_wal.clone();
            debug!("HBase: spawning tx upload thread");
            tasks.push(tokio::spawn(async move {
                debug!("HBase: calling put_bincode_cells_with_retry for tx");
                conn.put_bincode_cells_with_retry::<TransactionInfo>(
                    tx_table_name.as_str(),
                    &tx_cells,
                    use_tx_compression,
                    write_to_wal,
                )
                    .await
                    .map(TaskResult::BytesWritten)
                    .map_err(TaskError::from)
            }));
        }

        if !tx_by_addr_cells.is_empty() && !self.uploader_config.disable_tx_by_addr {
            let conn = self.connection.clone();
            let tx_by_addr_table_name = self.uploader_config.tx_by_addr_table_name.clone();
            let use_tx_by_addr_compression = self.uploader_config.use_tx_by_addr_compression.clone();
            let write_to_wal = self.uploader_config.hbase_write_to_wal.clone();
            debug!("HBase: spawning tx-by-addr upload thread");
            tasks.push(tokio::spawn(async move {
                debug!("HBase: calling put_protobuf_cells_with_retry tx-by-addr");
                conn.put_protobuf_cells_with_retry::<tx_by_addr::TransactionByAddr>(
                    tx_by_addr_table_name.as_str(),
                    &tx_by_addr_cells,
                    use_tx_by_addr_compression,
                    write_to_wal
                )
                    .await
                    .map(TaskResult::BytesWritten)
                    .map_err(TaskError::from)
                // info!("HBase: finished put_protobuf_cells_with_retry call for tx-by-addr");
            }));
        }

        let mut bytes_written = 0;
        let mut total_cached_transactions = 0;
        let mut maybe_first_err: Option<StorageError> = None;

        debug!("HBase: waiting for all upload threads to finish...");

        let results = futures::future::join_all(tasks).await;
        debug!("HBase: got upload results");
        for result in results {
            match result {
                Err(err) => {
                    debug!("HBase: got error result {:?}", err);
                    if maybe_first_err.is_none() {
                        maybe_first_err = Some(StorageError::TokioJoinError(err));
                    }
                }
                Ok(Err(err)) => {
                    debug!("HBase: got error result {:?}", err);
                    if maybe_first_err.is_none() {
                        match err {
                            TaskError::HBaseError(hbase_err) => {
                                maybe_first_err = Some(StorageError::StorageBackendError(Box::new(hbase_err)));
                            }
                            TaskError::MemcacheError(memcache_err) => {
                                maybe_first_err = Some(StorageError::CacheError(Box::new(memcache_err)));
                            }
                            TaskError::IoError(io_err) => {
                                maybe_first_err = Some(StorageError::IoError(io_err));
                            }
                            TaskError::EncodingError(enc_err) => {
                                maybe_first_err = Some(StorageError::EncodingError(enc_err));
                            }
                        }
                    }
                }
                Ok(Ok(task_result)) => {
                    match task_result {
                        TaskResult::BytesWritten(bytes) => bytes_written += bytes,
                        TaskResult::CachedTransactions(count) => total_cached_transactions += count,
                    }
                }
            }
        }

        if let Some(err) = maybe_first_err {
            debug!("HBase: returning upload error result {:?}", err);
            return Err(err.into());
        }

        if self.enable_full_tx_cache {
            info!("Cached {} transactions from slot {}",slot, total_cached_transactions);
        }

        let num_transactions = confirmed_block.transactions.len();

        // Store the block itself last, after all other metadata about the block has been
        // successfully stored.  This avoids partial uploaded blocks from becoming visible to
        // `get_confirmed_block()` and `get_confirmed_blocks()`
        let blocks_cells = [(
            slot_to_blocks_key(slot, self.uploader_config.use_md5_row_key_salt),
            confirmed_block.into()
        )];

        debug!("HBase: calling put_protobuf_cells_with_retry for blocks");

        if !self.uploader_config.disable_blocks {
            bytes_written += self
                .connection
                .put_protobuf_cells_with_retry::<generated::ConfirmedBlock>(
                    self.uploader_config.blocks_table_name.as_str(),
                    &blocks_cells,
                    self.uploader_config.use_blocks_compression,
                    self.uploader_config.hbase_write_to_wal
                )
                .await
                .map_err(|err| {
                    error!("HBase: failed to upload block: {:?}", err);
                    // err.into()
                    StorageError::StorageBackendError(Box::new(err))
                })?;
        }

        info!(
            "HBase: successfully uploaded block from slot {} [transactions: {}, bytes: {}]",
            slot, num_transactions, bytes_written
        );
        // datapoint_info!(
        //     "storage-hbase-upload-block",
        //     ("slot", slot, i64),
        //     ("transactions", num_transactions, i64),
        //     ("bytes", bytes_written, i64),
        // );
        Ok(())
    }

    fn should_include_in_tx_full(&self, address: &Pubkey) -> bool {
        if let Some(ref filter) = self.uploader_config.tx_full_filter {
            if filter.exclude {
                // If exclude is true, exclude the address if it's in the set.
                !filter.addrs.contains(address)
            } else {
                // If exclude is false, include the address only if it's in the set.
                filter.addrs.contains(address)
            }
        } else {
            true
        }
    }

    fn should_include_in_tx_by_addr(&self, address: &Pubkey) -> bool {
        if let Some(ref filter) = self.uploader_config.tx_by_addr_filter {
            if filter.exclude {
                // If exclude is true, exclude the address if it's in the set.
                !filter.addrs.contains(address)
            } else {
                // If exclude is false, include the address only if it's in the set.
                filter.addrs.contains(address)
            }
        } else {
            true
        }
    }

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter> {
        Box::new(self.clone())
    }
}