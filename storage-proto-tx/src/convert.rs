use {
    solana_transaction_status::{
        TransactionWithStatusMeta,
        ConfirmedTransactionWithStatusMeta,
    },
    std::{
        convert::{TryFrom},
    },
};

#[allow(clippy::derive_partial_eq_without_eq)]
pub mod confirmed_tx {
    include!(concat!(
        env!("OUT_DIR"),
        "/solana.storage.confirmed_tx.rs"
    ));
}

pub mod generated {
    pub use super::confirmed_tx::*;
    pub use solana_storage_proto::convert::generated::*;
}

impl From<ConfirmedTransactionWithStatusMeta> for generated::ConfirmedTransactionWithStatusMeta {
    fn from(value: ConfirmedTransactionWithStatusMeta) -> Self {
        Self {
            slot: value.slot as u64,
            tx_with_meta: Some(solana_storage_proto::convert::generated::ConfirmedTransaction::from(value.tx_with_meta)),
            block_time: value.block_time.map(|timestamp| generated::UnixTimestamp { timestamp }),
        }
    }
}

impl TryFrom<generated::ConfirmedTransactionWithStatusMeta> for ConfirmedTransactionWithStatusMeta {
    type Error = bincode::Error;

    fn try_from(value: generated::ConfirmedTransactionWithStatusMeta) -> Result<Self, Self::Error> {
        let tx_with_meta = match value.tx_with_meta {
            Some(tx) => TransactionWithStatusMeta::try_from(tx)?,
            None => return Err(bincode::Error::new(bincode::ErrorKind::Custom("transaction data is required".into()))),
        };

        Ok(ConfirmedTransactionWithStatusMeta {
            slot: value.slot,
            tx_with_meta,
            block_time: value.block_time.map(|generated::UnixTimestamp { timestamp }| timestamp),
        })
    }
}