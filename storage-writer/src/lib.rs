
use {
    async_trait::async_trait,
    log::*,
    solana_sdk::{
        clock::{
            Slot,
        },
        pubkey::Pubkey,
    },
    solana_transaction_status::{
        VersionedConfirmedBlock,
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
    CacheError(Box<dyn std::error::Error + Send>),

    #[error("Protobuf error: {0}")]
    EncodingError(prost::EncodeError),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;


#[async_trait]
pub trait LedgerStorageAdapter: Send + Sync {
    async fn upload_confirmed_block(
        &self,
        slot: Slot,
        confirmed_block: VersionedConfirmedBlock,
    ) -> Result<()>;

    fn should_include_in_tx_full(&self, address: &Pubkey) -> bool;
    fn should_include_in_tx_by_addr(&self, address: &Pubkey) -> bool;

    fn clone_box(&self) -> Box<dyn LedgerStorageAdapter>;
}

