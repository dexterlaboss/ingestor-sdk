#![allow(clippy::integer_arithmetic)]

use {
    crate::{
        hbase::{
            RowData,
        },
        deserializer::deserialize_protobuf_or_bincode_cell_data,
        tx_utils::{
            calculate_epoch,
            determine_transaction_type,
        },
        tx_cache::{
            get_cached_transaction,
        },
        storage_config::LedgerStorageConfig,
        hbase_error,
        connection,
        hbase,
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
        signature::Signature,
    },
    // dexter_storage_proto_tx::convert::*,
    dexter_storage_proto_tx::convert::{generated},
    solana_storage_proto::convert::{
        // generated,
        tx_by_addr
    },
    solana_transaction_status::{
        ConfirmedBlock, ConfirmedTransactionStatusWithSignature,
        ConfirmedTransactionWithStatusMeta,
        TransactionByAddrInfo,
        TransactionStatus,
    },
    solana_storage_reader::{
        Error, Result, LedgerStorageAdapter,
        StoredConfirmedBlock,
        StoredConfirmedTransactionWithStatusMeta,
        LegacyTransactionByAddrInfo,
    },
    solana_storage_utils::{
        tx_info::TransactionInfo,
        slot_to_blocks_key,
        slot_to_tx_by_addr_key,
        key_to_slot,
    },
    moka::sync::Cache,
    std::{
        convert::{TryInto},
        time::{Duration, Instant},
        boxed::Box,
    },
    memcache::Client as MemcacheClient,
    tokio::task,
};

// impl std::convert::From<hbase::Error> for Error {
//     fn from(err: hbase::Error) -> Self {
//         Self::StorageBackendError(Box::new(err))
//     }
// }

impl std::convert::From<hbase_error::Error> for Error {
    fn from(err: hbase_error::Error) -> Self {
        Self::StorageBackendError(Box::new(err))
    }
}

#[derive(Clone)]
pub struct LedgerStorage {
    // connection: hbase::HBaseConnection,
    connection: connection::HBaseConnection,
    cache: Option<Cache<Slot, RowData>>,
    use_md5_row_key_salt: bool,
    cache_client: Option<MemcacheClient>,
    disable_tx_fallback: bool,
    // TODO: Implement metrics
    // metrics: Arc<Metrics>,
    //----------------------
}

impl LedgerStorage {
    #[allow(dead_code)]
    pub async fn new(
        read_only: bool,
        timeout: Option<std::time::Duration>,
        // TODO: Implement metrics
        // metrics: Arc<Metrics>,
        //-----------------------
    ) -> Result<Self> {
        Self::new_with_config(LedgerStorageConfig {
            read_only,
            timeout,
            ..LedgerStorageConfig::default()
        },
                              // TODO: Implement metrics
                              // metrics.clone(),
                              //---------------------
        )
            .await
    }

    #[allow(dead_code)]
    pub async fn new_with_config(
        config: LedgerStorageConfig,
        // TODO: Implement metrics
        // metrics: Arc<Metrics>
        //-----------------------
    ) -> Result<Self> {
        debug!("Creating ledger storage instance with config: {:?}", config);
        let LedgerStorageConfig {
            read_only,
            timeout,
            address,
            block_cache,
            use_md5_row_key_salt,
            enable_full_tx_cache,
            disable_tx_fallback,
            cache_address,
        } = config;
        let connection = connection::HBaseConnection::new(
            address.as_str(),
            read_only,
            timeout,
        )
            .await?;

        let cache_client = if enable_full_tx_cache {
            if let Some(cache_addr) = cache_address {
                let cache_addr = format!("memcache://{}?timeout=1&protocol=ascii", cache_addr);

                let cache_addr_clone = cache_addr.clone();

                match task::spawn_blocking(move || MemcacheClient::connect(cache_addr_clone.as_str())).await {
                    Ok(Ok(client)) => Some(client),
                    Ok(Err(e)) => {
                        error!("Failed to connect to cache server at {}: {}", cache_addr, e);
                        None
                    },
                    Err(e) => {
                        error!("Tokio task join error while connecting to cache server: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let cache = if let Some(capacity) = block_cache {
            let lru_cache = Cache::new(capacity.get() as u64);
            Some(lru_cache)
        } else {
            None
        };

        Ok(Self {
            connection,
            cache,
            use_md5_row_key_salt,
            cache_client,
            disable_tx_fallback,
            // TODO: Implement metrics
            // metrics,
            //------------------------
        })
    }
}

#[async_trait]
impl LedgerStorageAdapter for LedgerStorage {
    /// Return the available slot that contains a block
    async fn get_first_available_block(&self) -> Result<Option<Slot>> {
        debug!("LedgerStorage::get_first_available_block request received");

        if self.use_md5_row_key_salt {
            return Ok(Some(0));
        }

        // inc_new_counter_debug!("storage-hbase-query", 1);
        let mut hbase = self.connection.client();
        let blocks = hbase.get_row_keys("blocks", None, None, 1, false).await?;
        if blocks.is_empty() {
            return Ok(None);
        }
        Ok(key_to_slot(&blocks[0]))
    }

    /// Fetch the next slots after the provided slot that contains a block
    ///
    /// start_slot: slot to start the search from (inclusive)
    /// limit: stop after this many slots have been found
    async fn get_confirmed_blocks(&self, start_slot: Slot, limit: usize) -> Result<Vec<Slot>> {
        debug!(
            "LedgerStorage::get_confirmed_blocks request received: {:?} {:?}",
            start_slot, limit
        );

        if self.use_md5_row_key_salt {
            return Ok(vec![]);
        }

        // inc_new_counter_debug!("storage-hbase-query", 1);
        let mut hbase = self.connection.client();
        let blocks = hbase
            .get_row_keys(
                "blocks",
                Some(slot_to_blocks_key(start_slot, false)),
                Some(slot_to_blocks_key(start_slot + limit as u64, false)), // None,
                limit as i64,
                false
            )
            .await?;
        Ok(blocks.into_iter().filter_map(|s| key_to_slot(&s)).collect())
    }

    /// Fetch the confirmed block from the desired slot
    async fn get_confirmed_block(&self, slot: Slot, use_cache: bool) -> Result<ConfirmedBlock> {
        debug!(
            "LedgerStorage::get_confirmed_block request received: {:?}",
            slot
        );
        // inc_new_counter_debug!("storage-hbase-query", 1);

        let start = Instant::now();
        let mut hbase = self.connection.client();
        let duration: Duration = start.elapsed();
        debug!("HBase connection took {:?}", duration);

        if use_cache {
            if let Some(cache) = &self.cache {
                if let Some(serialized_block) = cache.get(&slot) {
                    debug!("Using result from cache for {}", slot);
                    let block_cell_data =
                        deserialize_protobuf_or_bincode_cell_data::<StoredConfirmedBlock, generated::ConfirmedBlock>(
                            &serialized_block,
                            "blocks",
                            slot_to_blocks_key(slot, self.use_md5_row_key_salt)
                        )
                            .map_err(|err| match err {
                                hbase_error::Error::RowNotFound => Error::BlockNotFound(slot),
                                _ => err.into(),
                            })?;

                    let block: ConfirmedBlock = match block_cell_data {
                        hbase::CellData::Bincode(block) => block.into(),
                        hbase::CellData::Protobuf(block) => block.try_into().map_err(|_err| {
                            error!("Protobuf object is corrupted");
                            hbase_error::Error::ObjectCorrupt(format!("blocks/{}", slot_to_blocks_key(slot, self.use_md5_row_key_salt)))
                        })?,
                    };

                    return Ok(block.clone());
                }
            }
        }

        let block_cell_data_serialized = hbase
            .get_protobuf_or_bincode_cell_serialized::<StoredConfirmedBlock, generated::ConfirmedBlock>(
                "blocks",
                slot_to_blocks_key(slot, self.use_md5_row_key_salt),
            )
            .await
            .map_err(|err| {
                match err {
                    hbase_error::Error::RowNotFound => Error::BlockNotFound(slot),
                    _ => err.into(),
                }
            })?;

        let block_cell_data =
            deserialize_protobuf_or_bincode_cell_data::<StoredConfirmedBlock, generated::ConfirmedBlock>(
                &block_cell_data_serialized,
                "blocks",
                slot_to_blocks_key(slot, self.use_md5_row_key_salt),
            )?;

        let block: ConfirmedBlock = match block_cell_data {
            hbase::CellData::Bincode(block) => block.into(),
            hbase::CellData::Protobuf(block) => block.try_into().map_err(|_err| {
                error!("Protobuf object is corrupted");
                hbase_error::Error::ObjectCorrupt(format!("blocks/{}", slot_to_blocks_key(slot, self.use_md5_row_key_salt)))
            })?,
        };

        if use_cache {
            if let Some(cache) = &self.cache {
                debug!("Storing block {} in cache", slot);
                cache.insert(slot, block_cell_data_serialized.clone());
            }
        }

        Ok(block)
    }

    async fn get_signature_status(&self, signature: &Signature) -> Result<TransactionStatus> {
        debug!(
            "LedgerStorage::get_signature_status request received: {:?}",
            signature
        );
        // inc_new_counter_debug!("storage-hbase-query", 1);
        let mut hbase = self.connection.client();
        let transaction_info = hbase
            .get_bincode_cell::<TransactionInfo>("tx", signature.to_string())
            .await
            .map_err(|err| match err {
                // hbase::Error::RowNotFound => Error::SignatureNotFound,
                hbase_error::Error::RowNotFound => Error::SignatureNotFound,
                _ => err.into(),
            })?;
        Ok(transaction_info.into())
    }

    async fn get_full_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<ConfirmedTransactionWithStatusMeta>> {
        debug!(
            "LedgerStorage::get_full_transaction request received: {:?}",
            signature
        );
        // inc_new_counter_debug!("storage-hbase-query", 1);

        let mut hbase = self.connection.client();

        let tx_cell_data = hbase
            .get_protobuf_or_bincode_cell::<StoredConfirmedTransactionWithStatusMeta, generated::ConfirmedTransactionWithStatusMeta>(
                "tx_full",
                signature.to_string(),
            )
            .await
            .map_err(|err| match err {
                hbase_error::Error::RowNotFound => Error::SignatureNotFound,
                _ => err.into(),
            })?;

        Ok(match tx_cell_data {
            hbase::CellData::Bincode(tx) => Some(tx.into()),
            hbase::CellData::Protobuf(tx) => Some(tx.try_into().map_err(|_err| {
                error!("Protobuf object is corrupted");
                hbase_error::Error::ObjectCorrupt(format!("tx_full/{}", signature.to_string()))
            })?),
        })
    }

    /// Fetch a confirmed transaction
    async fn get_confirmed_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<ConfirmedTransactionWithStatusMeta>> {
        debug!(
            "LedgerStorage::get_confirmed_transaction request received: {:?}",
            signature
        );
        debug!("LedgerStorage::get_confirmed_transaction using address: {:?}", self.connection);

        // let mut source = "tx";
        let _tx_type;
        let _epoch: u64;

        if let Some(cache_client) = &self.cache_client {
            // match get_cached_transaction::<generated_util::ConfirmedTransactionWithStatusMeta>(cache_client, signature).await {
            match get_cached_transaction::<generated::ConfirmedTransactionWithStatusMeta>(cache_client, signature).await {
                Ok(Some(tx)) => {
                    let confirmed_tx: ConfirmedTransactionWithStatusMeta = match tx.try_into() {
                        Ok(val) => val,
                        Err(_) => {
                            warn!("Cached protobuf object is corrupted for transaction {}", signature.to_string());
                            return Ok(None);
                        }
                    };

                    _epoch = calculate_epoch(confirmed_tx.slot);

                    // source = "cache";
                    _tx_type = determine_transaction_type(&confirmed_tx.tx_with_meta);
                    // TODO: Implement metrics
                    // self.metrics.record_transaction(source, _epoch, _tx_type);
                    //-------------------------

                    return Ok(Some(confirmed_tx));
                }
                Ok(None) => {
                    debug!("Transaction {} not found in cache", signature);
                }
                Err(e) => {
                    warn!("Failed to read transaction from cache for {}: {:?}",signature, e);
                }
            }
        }

        // inc_new_counter_debug!("storage-hbase-query", 1);

        if let Ok(Some(full_tx)) = self.get_full_transaction(signature).await {
            _epoch = calculate_epoch(full_tx.slot);

            // source = "tx_full";
            _tx_type = determine_transaction_type(&full_tx.tx_with_meta);
            // TODO: Implement metrics
            // self.metrics.record_transaction(source, epoch, _tx_type);
            //------------------------

            return Ok(Some(full_tx));
        } else {
            debug!("Transaction not found in the full_tx table");
        }

        debug!("disable_tx_fallback: {:?}", self.disable_tx_fallback);

        if self.disable_tx_fallback {
            debug!("Fallback to tx table is disabled");
            return Ok(None);
        }

        debug!("Looking for transaction in tx table");

        let mut hbase = self.connection.client();

        // Figure out which block the transaction is located in
        let TransactionInfo { slot, index, .. } = hbase
            .get_bincode_cell("tx", signature.to_string())
            .await
            .map_err(|err| match err {
                hbase_error::Error::RowNotFound => Error::SignatureNotFound,
                _ => Error::StorageBackendError(Box::new(err)),
            })?;

        _epoch = calculate_epoch(slot);

        // Load the block and return the transaction
        let block = self.get_confirmed_block(slot, true).await?;
        match block.transactions.into_iter().nth(index as usize) {
            None => {
                warn!("Transaction info for {} is corrupt", signature);
                Ok(None)
            }
            Some(tx_with_meta) => {
                if tx_with_meta.transaction_signature() != signature {
                    warn!(
                        "Transaction info or confirmed block for {} is corrupt",
                        signature
                    );
                    Ok(None)
                } else {
                    _tx_type = determine_transaction_type(&tx_with_meta); // Determine the transaction type
                    // TODO: Implement metrics
                    // self.metrics.record_transaction(source, _epoch, _tx_type);
                    //-------------------------

                    Ok(Some(ConfirmedTransactionWithStatusMeta {
                        slot,
                        tx_with_meta,
                        block_time: block.block_time,
                    }))
                }
            }
        }
    }

    async fn get_confirmed_signatures_for_address(
        &self,
        address: &Pubkey,
        before_signature: Option<&Signature>,
        until_signature: Option<&Signature>,
        limit: usize,
    ) -> Result<
        Vec<(
            ConfirmedTransactionStatusWithSignature,
            u32,
        )>,
    > {
        // info!(
        //     "LedgerStorage::get_confirmed_signatures_for_address: {:?}",
        //     address
        // );
        // info!("Using signature range [before: {:?}, until: {:?}]", before_signature.clone(), until_signature.clone());

        // inc_new_counter_debug!("storage-hbase-query", 1);
        let mut hbase = self.connection.client();
        let address_prefix = format!("{address}/");

        // Figure out where to start listing from based on `before_signature`
        let (first_slot, before_transaction_index, before_fallback) = match before_signature {
            None => (Slot::MAX, 0, false),
            Some(before_signature) => {
                // Try fetching from `tx` first
                match hbase.get_bincode_cell("tx", before_signature.to_string()).await {
                    Ok(TransactionInfo { slot, index, .. }) => (slot, index, false),
                    // Fallback to `tx_full` if `tx` is not found
                    Err(hbase_error::Error::RowNotFound) => {
                        match self.get_full_transaction(before_signature).await? {
                            Some(full_transaction) => (full_transaction.slot, 0, true),
                            None => return Ok(vec![]),
                        }
                    },
                    Err(err) => return Err(err.into()),
                }
            }
        };

        debug!("Got starting slot: {:?}, index: {:?}, using tx_full fallback: {:?}",
            first_slot.clone(),
            before_transaction_index.clone(),
            before_fallback
        );

        // Figure out where to end listing from based on `until_signature`
        let (last_slot, until_transaction_index, until_fallback) = match until_signature {
            None => (0, u32::MAX, false),
            Some(until_signature) => {
                // Try fetching from `tx` first
                match hbase.get_bincode_cell("tx", until_signature.to_string()).await {
                    Ok(TransactionInfo { slot, index, .. }) => (slot, index, false),
                    // Fallback to `tx_full` if `tx` is not found
                    Err(hbase_error::Error::RowNotFound) => {
                        match self.get_full_transaction(until_signature).await? {
                            Some(full_transaction) => (full_transaction.slot, 0, true),
                            None => return Ok(vec![]),
                        }
                    },
                    Err(err) => return Err(err.into()),
                }
            }
        };

        debug!("Got ending slot: {:?}, index: {:?}, using tx_full fallback: {:?}",
            last_slot.clone(),
            until_transaction_index.clone(),
            until_fallback
        );

        let mut infos = vec![];

        debug!("Getting the starting slot length from tx-by-addr");

        let starting_slot_tx_len = hbase
            .get_protobuf_or_bincode_cell::<Vec<LegacyTransactionByAddrInfo>, tx_by_addr::TransactionByAddr>(
                "tx-by-addr",
                format!("{}{}", address_prefix, slot_to_tx_by_addr_key(first_slot)),
            )
            .await
            .map(|cell_data| {
                match cell_data {
                    hbase::CellData::Bincode(tx_by_addr) => tx_by_addr.len(),
                    hbase::CellData::Protobuf(tx_by_addr) => tx_by_addr.tx_by_addrs.len(),
                }
            })
            .unwrap_or(0);

        debug!("Got starting slot tx len: {:?}", starting_slot_tx_len);

        // Return the next tx-by-addr data of amount `limit` plus extra to account for the largest
        // number that might be flitered out
        let tx_by_addr_data = hbase
            .get_row_data(
                "tx-by-addr",
                Some(format!(
                    "{}{}",
                    address_prefix,
                    slot_to_tx_by_addr_key(first_slot),
                )),
                Some(format!(
                    "{}{}",
                    address_prefix,
                    slot_to_tx_by_addr_key(last_slot.saturating_sub(1)),
                )),
                limit as i64 + starting_slot_tx_len as i64,
            )
            .await?;

        debug!("Loaded {:?} tx-by-addr entries", tx_by_addr_data.len());

        'outer: for (row_key, data) in tx_by_addr_data {
            let slot = !key_to_slot(&row_key[address_prefix.len()..]).ok_or_else(|| {
                hbase_error::Error::ObjectCorrupt(format!(
                    "Failed to convert key to slot: tx-by-addr/{row_key}"
                ))
            })?;

            debug!("Deserializing tx-by-addr result data");

            let deserialized_cell_data = deserialize_protobuf_or_bincode_cell_data::<
                Vec<LegacyTransactionByAddrInfo>,
                tx_by_addr::TransactionByAddr,
            >(&data, "tx-by-addr", row_key.clone())?;

            let mut cell_data: Vec<TransactionByAddrInfo> = match deserialized_cell_data {
                hbase::CellData::Bincode(tx_by_addr) => {
                    tx_by_addr.into_iter().map(|legacy| legacy.into()).collect()
                }
                hbase::CellData::Protobuf(tx_by_addr) => {
                    tx_by_addr.try_into().map_err(|error| {
                        hbase_error::Error::ObjectCorrupt(format!(
                            "Failed to deserialize: {}: tx-by-addr/{}",
                            error,
                            row_key.clone()
                        ))
                    })?
                }
            };

            cell_data.reverse();

            debug!("Filtering the result data");

            for tx_by_addr_info in cell_data.into_iter() {
                debug!("Checking result [slot: {:?}, index: {:?}], signature: {:?}", slot, tx_by_addr_info.index, tx_by_addr_info.signature);

                // Filter out records before `before_transaction_index`
                if !before_fallback && slot == first_slot && tx_by_addr_info.index >= before_transaction_index {
                    debug!("Skipping transaction before [slot: {:?}, index: {:?}], signature: {:?}", slot, tx_by_addr_info.index, tx_by_addr_info.signature);
                    continue;
                }

                // Filter out records after `until_transaction_index` unless fallback was used
                if !until_fallback && slot == last_slot && tx_by_addr_info.index <= until_transaction_index {
                    debug!("Skipping transaction until [slot: {:?}, index: {:?}], signature: {:?}", slot, tx_by_addr_info.index, tx_by_addr_info.signature);
                    continue;
                }

                infos.push((
                    ConfirmedTransactionStatusWithSignature {
                        signature: tx_by_addr_info.signature,
                        slot,
                        err: tx_by_addr_info.err,
                        memo: tx_by_addr_info.memo,
                        block_time: tx_by_addr_info.block_time,
                    },
                    tx_by_addr_info.index,
                ));
                // Respect limit
                debug!("Checking the limit: {:?}/{:?}", infos.len(), limit);
                if infos.len() >= limit {
                    debug!("Limit was reached, exiting loop");
                    break 'outer;
                }
            }
        }

        debug!("Returning {:?} result entries", infos.len());

        Ok(infos)
    }

    async fn get_latest_stored_slot(&self) -> Result<Slot> {
        // inc_new_counter_debug!("storage-hbase-query", 1);
        let mut hbase = self.connection.client();
        match hbase.get_last_row_key("blocks").await {
            Ok(last_row_key) => {
                match key_to_slot(&last_row_key) {
                    Some(slot) => Ok(slot),
                    None => Err(Error::StorageBackendError(Box::new(hbase_error::Error::ObjectCorrupt(format!(
                        "Failed to parse row key '{}' as slot number",
                        last_row_key
                    ))))),
                }
            },
            Err(hbase_error::Error::RowNotFound) => {
                // If the table is empty, return a default value (e.g., first_slot - 1)
                Ok(Slot::default())
            },
            Err(e) => Err(Error::StorageBackendError(Box::new(e))),
        }
    }

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter> {
        Box::new(self.clone())
    }
}
