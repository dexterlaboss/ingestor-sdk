#![allow(clippy::integer_arithmetic)]

use {
    solana_bigtable_shared::{
        CredentialType,
        DEFAULT_INSTANCE_NAME,
        DEFAULT_APP_PROFILE_ID,
    },
    async_trait::async_trait,
    log::*,
    solana_clock::{
        Slot,
    },
    solana_pubkey::{
        Pubkey,
    },
    solana_reserved_account_keys::ReservedAccountKeys,
    solana_storage_proto::convert::{generated, tx_by_addr},
    solana_transaction_status::{
        extract_and_fmt_memos,
        TransactionByAddrInfo,
        VersionedConfirmedBlock,
        VersionedTransactionWithStatusMeta,
    },
    solana_storage_writer::{
        Error, Result, LedgerStorageAdapter,
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
        time::Duration,
    },
};

mod bigtable;


impl std::convert::From<bigtable::Error> for Error {
    fn from(err: bigtable::Error) -> Self {
        Self::StorageBackendError(Box::new(err))
    }
}


#[derive(Debug)]
pub struct LedgerStorageConfig {
    pub read_only: bool,
    pub timeout: Option<std::time::Duration>,
    pub credential_type: CredentialType,
    pub instance_name: String,
    pub app_profile_id: String,
}

impl Default for LedgerStorageConfig {
    fn default() -> Self {
        Self {
            read_only: true,
            timeout: None,
            credential_type: CredentialType::Filepath(None),
            instance_name: DEFAULT_INSTANCE_NAME.to_string(),
            app_profile_id: DEFAULT_APP_PROFILE_ID.to_string(),
        }
    }
}


#[derive(Clone)]
pub struct LedgerStorage {
    connection: bigtable::BigTableConnection,
    // stats: Arc<LedgerStorageStats>,
}

impl LedgerStorage {
    pub async fn new(
        read_only: bool,
        timeout: Option<std::time::Duration>,
        credential_path: Option<String>,
    ) -> Result<Self> {
        Self::new_with_config(LedgerStorageConfig {
            read_only,
            timeout,
            credential_type: CredentialType::Filepath(credential_path),
            ..LedgerStorageConfig::default()
        })
            .await
    }

    pub fn new_for_emulator(
        instance_name: &str,
        app_profile_id: &str,
        endpoint: &str,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        // let stats = Arc::new(LedgerStorageStats::default());
        Ok(Self {
            connection: bigtable::BigTableConnection::new_for_emulator(
                instance_name,
                app_profile_id,
                endpoint,
                timeout,
            )?,
            // stats,
        })
    }

    pub async fn new_with_config(config: LedgerStorageConfig) -> Result<Self> {
        // let stats = Arc::new(LedgerStorageStats::default());
        let LedgerStorageConfig {
            read_only,
            timeout,
            instance_name,
            app_profile_id,
            credential_type,
        } = config;
        let connection = bigtable::BigTableConnection::new(
            instance_name.as_str(),
            app_profile_id.as_str(),
            read_only,
            timeout,
            credential_type,
        )
            .await?;
        Ok(Self { /*stats,*/ connection })
    }

    pub async fn new_with_stringified_credential(credential: String) -> Result<Self> {
        Self::new_with_config(LedgerStorageConfig {
            credential_type: CredentialType::Stringified(credential),
            ..LedgerStorageConfig::default()
        })
            .await
    }
}

#[async_trait]
impl LedgerStorageAdapter for LedgerStorage {
    /// Upload a new confirmed block and associated meta data.
    async fn upload_confirmed_block(
        &self,
        slot: Slot,
        confirmed_block: VersionedConfirmedBlock,
    ) -> Result<()> {
        trace!(
            "LedgerStorage::upload_confirmed_block request received: {:?}",
            slot
        );
        let mut by_addr: HashMap<&Pubkey, Vec<TransactionByAddrInfo>> = HashMap::new();

        let reserved_account_keys = ReservedAccountKeys::new_all_activated();
        let mut tx_cells = Vec::with_capacity(confirmed_block.transactions.len());
        for (index, transaction_with_meta) in confirmed_block.transactions.iter().enumerate() {
            let VersionedTransactionWithStatusMeta { meta, transaction } = transaction_with_meta;
            let err = meta.status.clone().err();
            let index = index as u32;
            let signature = transaction.signatures[0];
            let memo = extract_and_fmt_memos(transaction_with_meta);

            for address in transaction_with_meta.account_keys().iter() {
                // if !is_sysvar_id(address) {
                if !reserved_account_keys.is_reserved(address) {
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

        if !tx_cells.is_empty() {
            let conn = self.connection.clone();
            tasks.push(tokio::spawn(async move {
                conn.put_bincode_cells_with_retry::<TransactionInfo>("tx", &tx_cells)
                    .await
            }));
        }

        if !tx_by_addr_cells.is_empty() {
            let conn = self.connection.clone();
            tasks.push(tokio::spawn(async move {
                conn.put_protobuf_cells_with_retry::<tx_by_addr::TransactionByAddr>(
                    "tx-by-addr",
                    &tx_by_addr_cells,
                )
                .await
            }));
        }

        let mut _bytes_written = 0;
        let mut maybe_first_err: Option<Error> = None;

        let results = futures::future::join_all(tasks).await;
        for result in results {
            match result {
                Err(err) => {
                    if maybe_first_err.is_none() {
                        maybe_first_err = Some(Error::TokioJoinError(err));
                    }
                }
                Ok(Err(err)) => {
                    if maybe_first_err.is_none() {
                        maybe_first_err = Some(Error::StorageBackendError(Box::new(err)));
                    }
                }
                Ok(Ok(bytes)) => {
                    _bytes_written += bytes;
                }
            }
        }

        if let Some(err) = maybe_first_err {
            return Err(err);
        }

        let _num_transactions = confirmed_block.transactions.len();

        // Store the block itself last, after all other metadata about the block has been
        // successfully stored.  This avoids partial uploaded blocks from becoming visible to
        // `get_confirmed_block()` and `get_confirmed_blocks()`
        let blocks_cells = [(slot_to_blocks_key(slot, false), confirmed_block.into())];
        _bytes_written += self
            .connection
            .put_protobuf_cells_with_retry::<generated::ConfirmedBlock>("blocks", &blocks_cells)
            .await?;
        // datapoint_info!(
        //     "storage-bigtable-upload-block",
        //     ("slot", slot, i64),
        //     ("transactions", num_transactions, i64),
        //     ("bytes", bytes_written, i64),
        // );
        Ok(())
    }

    fn should_include_in_tx_full(&self, address: &Pubkey) -> bool {
        true
    }

    fn should_include_in_tx_by_addr(&self, address: &Pubkey) -> bool {
        true
    }

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter> {
        Box::new(self.clone())
    }
}
