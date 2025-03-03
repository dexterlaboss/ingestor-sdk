use {
    log::*,
    solana_clock::{
        Slot,
    },
    std::{
        boxed::Box,
    },
    thiserror::Error,
    tokio::task::JoinError,
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

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}