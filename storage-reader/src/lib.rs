
use {
    async_trait::async_trait,
    log::*,
    serde::{Deserialize, Serialize},
    solana_sdk::{
        clock::{Slot, UnixTimestamp},
        pubkey::Pubkey,
        signature::Signature,
        message::v0::LoadedAddresses,
        deserialize_utils::default_on_eof,
        transaction::{TransactionError, VersionedTransaction},
    },
    solana_transaction_status::{
        ConfirmedBlock,
        ConfirmedTransactionStatusWithSignature,
        ConfirmedTransactionWithStatusMeta,
        TransactionStatus,
        TransactionStatusMeta,
        TransactionWithStatusMeta,
        VersionedTransactionWithStatusMeta,
        TransactionByAddrInfo,
        Reward,
    },
    std::{
        boxed::Box,
    },
    thiserror::Error,
    tokio::task::JoinError,
};

#[macro_use]
extern crate serde_derive;

pub mod compression;

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
    CacheError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;


// A serialized `StoredConfirmedBlock` is stored in the `block` table
//
// StoredConfirmedBlock holds the same contents as ConfirmedBlock, but is slightly compressed and avoids
// some serde JSON directives that cause issues with bincode
//
// Note: in order to continue to support old bincode-serialized bigtable entries, if new fields are
// added to ConfirmedBlock, they must either be excluded or set to `default_on_eof` here
//
#[derive(Serialize, Deserialize)]
pub struct StoredConfirmedBlock {
    previous_blockhash: String,
    blockhash: String,
    parent_slot: Slot,
    transactions: Vec<StoredConfirmedBlockTransaction>,
    rewards: StoredConfirmedBlockRewards,
    // num_partitions: Option<u64>,
    block_time: Option<UnixTimestamp>,
    #[serde(deserialize_with = "default_on_eof")]
    block_height: Option<u64>,
}

#[derive(Serialize, Deserialize)]
pub struct StoredConfirmedTransactionWithStatusMeta {
    pub slot: Slot,
    pub tx_with_meta: StoredConfirmedBlockTransaction,
    pub block_time: Option<UnixTimestamp>,
}

impl From<ConfirmedTransactionWithStatusMeta> for StoredConfirmedTransactionWithStatusMeta {
    fn from(value: ConfirmedTransactionWithStatusMeta) -> Self {
        Self {
            slot: value.slot,
            tx_with_meta: value.tx_with_meta.into(),
            block_time: value.block_time,
        }
    }
}

impl From<StoredConfirmedTransactionWithStatusMeta> for ConfirmedTransactionWithStatusMeta {
    fn from(value: StoredConfirmedTransactionWithStatusMeta) -> Self {
        Self {
            slot: value.slot,
            tx_with_meta: value.tx_with_meta.into(),
            block_time: value.block_time,
        }
    }
}

impl From<ConfirmedBlock> for StoredConfirmedBlock {
    fn from(confirmed_block: ConfirmedBlock) -> Self {
        let ConfirmedBlock {
            previous_blockhash,
            blockhash,
            parent_slot,
            transactions,
            rewards,
            num_partitions: _num_partitions,
            block_time,
            block_height,
        } = confirmed_block;

        Self {
            previous_blockhash,
            blockhash,
            parent_slot,
            transactions: transactions.into_iter().map(|tx| tx.into()).collect(),
            rewards: rewards.into_iter().map(|reward| reward.into()).collect(),
            block_time,
            block_height,
        }
    }
}

impl From<StoredConfirmedBlock> for ConfirmedBlock {
    fn from(confirmed_block: StoredConfirmedBlock) -> Self {
        let StoredConfirmedBlock {
            previous_blockhash,
            blockhash,
            parent_slot,
            transactions,
            rewards,
            block_time,
            block_height,
        } = confirmed_block;

        Self {
            previous_blockhash,
            blockhash,
            parent_slot,
            transactions: transactions.into_iter().map(|tx| tx.into()).collect(),
            rewards: rewards.into_iter().map(|reward| reward.into()).collect(),
            num_partitions: None,
            block_time,
            block_height,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StoredConfirmedBlockTransaction {
    transaction: VersionedTransaction,
    meta: Option<StoredConfirmedBlockTransactionStatusMeta>,
}

impl From<TransactionWithStatusMeta> for StoredConfirmedBlockTransaction {
    fn from(value: TransactionWithStatusMeta) -> Self {
        match value {
            TransactionWithStatusMeta::MissingMetadata(transaction) => Self {
                transaction: VersionedTransaction::from(transaction),
                meta: None,
            },
            TransactionWithStatusMeta::Complete(VersionedTransactionWithStatusMeta {
                                                    transaction,
                                                    meta,
                                                }) => Self {
                transaction,
                meta: Some(meta.into()),
            },
        }
    }
}

impl From<StoredConfirmedBlockTransaction> for TransactionWithStatusMeta {
    fn from(tx_with_meta: StoredConfirmedBlockTransaction) -> Self {
        let StoredConfirmedBlockTransaction { transaction, meta } = tx_with_meta;
        match meta {
            None => Self::MissingMetadata(
                transaction
                    .into_legacy_transaction()
                    .expect("versioned transactions always have meta"),
            ),
            Some(meta) => Self::Complete(VersionedTransactionWithStatusMeta {
                transaction,
                meta: meta.into(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StoredConfirmedBlockTransactionStatusMeta {
    err: Option<TransactionError>,
    fee: u64,
    pre_balances: Vec<u64>,
    post_balances: Vec<u64>,
}

impl From<StoredConfirmedBlockTransactionStatusMeta> for TransactionStatusMeta {
    fn from(value: StoredConfirmedBlockTransactionStatusMeta) -> Self {
        let StoredConfirmedBlockTransactionStatusMeta {
            err,
            fee,
            pre_balances,
            post_balances,
        } = value;
        let status = match &err {
            None => Ok(()),
            Some(err) => Err(err.clone()),
        };
        Self {
            status,
            fee,
            pre_balances,
            post_balances,
            inner_instructions: None,
            log_messages: None,
            pre_token_balances: None,
            post_token_balances: None,
            rewards: None,
            loaded_addresses: LoadedAddresses::default(),
            return_data: None,
            compute_units_consumed: None,
        }
    }
}

impl From<TransactionStatusMeta> for StoredConfirmedBlockTransactionStatusMeta {
    fn from(value: TransactionStatusMeta) -> Self {
        let TransactionStatusMeta {
            status,
            fee,
            pre_balances,
            post_balances,
            ..
        } = value;
        Self {
            err: status.err(),
            fee,
            pre_balances,
            post_balances,
        }
    }
}

pub type StoredConfirmedBlockRewards = Vec<StoredConfirmedBlockReward>;

#[derive(Serialize, Deserialize)]
pub struct StoredConfirmedBlockReward {
    pubkey: String,
    lamports: i64,
}

impl From<StoredConfirmedBlockReward> for Reward {
    fn from(value: StoredConfirmedBlockReward) -> Self {
        let StoredConfirmedBlockReward { pubkey, lamports } = value;
        Self {
            pubkey,
            lamports,
            post_balance: 0,
            reward_type: None,
            commission: None,
        }
    }
}

impl From<Reward> for StoredConfirmedBlockReward {
    fn from(value: Reward) -> Self {
        let Reward {
            pubkey, lamports, ..
        } = value;
        Self { pubkey, lamports }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyTransactionByAddrInfo {
    pub signature: Signature,          // The transaction signature
    pub err: Option<TransactionError>, // None if the transaction executed successfully
    pub index: u32,                    // Where the transaction is located in the block
    pub memo: Option<String>,          // Transaction memo
}

impl From<LegacyTransactionByAddrInfo> for TransactionByAddrInfo {
    fn from(legacy: LegacyTransactionByAddrInfo) -> Self {
        let LegacyTransactionByAddrInfo {
            signature,
            err,
            index,
            memo,
        } = legacy;

        Self {
            signature,
            err,
            index,
            memo,
            block_time: None,
        }
    }
}

#[async_trait]
pub trait LedgerStorageAdapter: Send + Sync {
    async fn get_first_available_block(&self) -> Result<Option<Slot>>;

    async fn get_confirmed_blocks(&self, start_slot: Slot, limit: usize) -> Result<Vec<Slot>>;

    async fn get_confirmed_block(&self, slot: Slot, use_cache: bool) -> Result<ConfirmedBlock>;

    async fn get_signature_status(&self, signature: &Signature) -> Result<TransactionStatus>;

    async fn get_full_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<ConfirmedTransactionWithStatusMeta>>;

    async fn get_confirmed_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<ConfirmedTransactionWithStatusMeta>>;

    async fn get_confirmed_signatures_for_address(
        &self,
        address: &Pubkey,
        before_signature: Option<&Signature>,
        until_signature: Option<&Signature>,
        limit: usize,
    ) -> Result<Vec<(ConfirmedTransactionStatusWithSignature, u32)>>;

    async fn get_latest_stored_slot(&self) -> Result<Slot>;

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter>;
}
