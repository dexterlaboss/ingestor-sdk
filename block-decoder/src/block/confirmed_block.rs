
use {
    crate::{
        errors::{
            decode_error::DecodeError,
        },
        decodable::Decodable,
        transaction::{
            versioned_transaction::VersionedTransactionWithStatusMeta,
        },
        block::{
            encoded_block::EncodedTransactionWithStatusMeta,
            ui_block::UiConfirmedBlock,
        },
        transaction::{
            transaction::{Transaction},
        }
    },
    solana_sdk::{
        clock::{Slot, UnixTimestamp},
    },
    solana_transaction_status::{
        UiTransactionEncoding,
        BlockEncodingOptions,
        TransactionDetails,
        Rewards,
    },
    serde_derive::{Serialize,Deserialize},
};

#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmedBlock {
    pub previous_blockhash: String,
    pub blockhash: String,
    pub parent_slot: Slot,
    pub transactions: Vec<TransactionWithStatusMeta>,
    pub rewards: Rewards,
    pub num_partitions: Option<u64>,
    pub block_time: Option<UnixTimestamp>,
    pub block_height: Option<u64>,
}

impl ConfirmedBlock {
    pub fn decode_with_options(
        ui_confirmed_block: UiConfirmedBlock,
        encoding: UiTransactionEncoding,
        options: BlockEncodingOptions,
    ) -> Result<Self, DecodeError> {
        let transactions = match options.transaction_details {
            TransactionDetails::Full => {
                let transactions = ui_confirmed_block
                    .transactions
                    .ok_or(DecodeError::InvalidEncoding)?
                    .into_iter()
                    .map(|encoded_tx_with_meta| {
                        TransactionWithStatusMeta::decode(encoded_tx_with_meta, encoding)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                transactions
            }
            TransactionDetails::Signatures => {
                let _signatures = ui_confirmed_block
                    .signatures
                    .ok_or(DecodeError::InvalidEncoding)?;
                // Implement a method or mechanism to retrieve transactions using signatures
                return Err(DecodeError::NotImplemented);
            }
            TransactionDetails::None => Vec::new(),
            TransactionDetails::Accounts => {
                let transactions = ui_confirmed_block
                    .transactions
                    .ok_or(DecodeError::InvalidEncoding)?
                    .into_iter()
                    .map(|encoded_tx_with_meta| {
                        TransactionWithStatusMeta::decode(encoded_tx_with_meta, encoding)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                transactions
            }
        };

        Ok(ConfirmedBlock {
            previous_blockhash: ui_confirmed_block.previous_blockhash,
            blockhash: ui_confirmed_block.blockhash,
            parent_slot: ui_confirmed_block.parent_slot,
            transactions,
            rewards: ui_confirmed_block
                .rewards
                .unwrap_or_default(),
            num_partitions: ui_confirmed_block.num_reward_partitions,
            block_time: ui_confirmed_block.block_time,
            block_height: ui_confirmed_block.block_height,
        })
    }
}


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TransactionWithStatusMeta {
    // Very old transactions may be missing metadata
    MissingMetadata(Transaction),
    // Versioned stored transaction always have metadata
    Complete(VersionedTransactionWithStatusMeta),
}

impl TransactionWithStatusMeta {
    pub fn decode(
        encoded: EncodedTransactionWithStatusMeta,
        encoding: UiTransactionEncoding,
    ) -> Result<Self, DecodeError> {
        match encoded.meta {
            Some(_) => {
                let complete = VersionedTransactionWithStatusMeta::decode(encoded, encoding /*, None*/)?;
                Ok(Self::Complete(complete))
            },
            None => {
                let transaction = Transaction::decode(&encoded.transaction)?;
                Ok(Self::MissingMetadata(transaction))
            }
        }
    }
}