
use {
    crate::{
        block::{
            confirmed_block::{
                ConfirmedBlock,
                TransactionWithStatusMeta
            },
        },
        transaction::{
            versioned_transaction::VersionedTransactionWithStatusMeta,
        },
    },
    solana_sdk::{
        clock::{Slot, UnixTimestamp},
    },
    solana_transaction_status::{
        Rewards,
    },
    thiserror::Error,
};


// Confirmed block with type guarantees that transaction metadata
// is always present. Used for uploading to HBase.
#[derive(Clone, Debug, PartialEq)]
pub struct VersionedConfirmedBlock {
    pub previous_blockhash: String,
    pub blockhash: String,
    pub parent_slot: Slot,
    pub transactions: Vec<VersionedTransactionWithStatusMeta>,
    pub rewards: Rewards,
    pub num_partitions: Option<u64>,
    pub block_time: Option<UnixTimestamp>,
    pub block_height: Option<u64>,
}


impl TryFrom<ConfirmedBlock> for VersionedConfirmedBlock {
    type Error = ConvertBlockError;

    fn try_from(block: ConfirmedBlock) -> Result<Self, Self::Error> {
        let expected_transaction_count = block.transactions.len();

        let txs: Vec<_> = block
            .transactions
            .into_iter()
            .filter_map(|tx| match tx {
                TransactionWithStatusMeta::MissingMetadata(_) => None,
                TransactionWithStatusMeta::Complete(tx) => Some(tx),
            })
            .collect();

        if txs.len() != expected_transaction_count {
            return Err(ConvertBlockError::TransactionsMissing(
                expected_transaction_count,
                txs.len(),
            ));
        }

        Ok(Self {
            previous_blockhash: block.previous_blockhash,
            blockhash: block.blockhash,
            parent_slot: block.parent_slot,
            transactions: txs,
            rewards: block.rewards,
            num_partitions: block.num_partitions,
            block_time: block.block_time,
            block_height: block.block_height,
        })
    }
}

#[derive(Debug, Error)]
pub enum ConvertBlockError {
    #[error("transactions missing after converted, before: {0}, after: {1}")]
    TransactionsMissing(usize, usize),
}

impl From<VersionedConfirmedBlock> for solana_transaction_status::VersionedConfirmedBlock {
    fn from(block: VersionedConfirmedBlock) -> Self {
        Self {
            previous_blockhash: block.previous_blockhash,
            blockhash: block.blockhash,
            parent_slot: block.parent_slot,
            transactions: block.transactions.into_iter().map(Into::into).collect(),
            rewards: block.rewards,
            num_partitions: block.num_partitions,
            block_time: block.block_time,
            block_height: block.block_height,
        }
    }
}
