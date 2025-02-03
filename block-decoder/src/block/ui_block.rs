
use {
    crate::{
        block::{
            encoded_block::{EncodedTransactionWithStatusMeta, EncodedConfirmedBlock},
        },
    },
    solana_sdk::{
        clock::{Slot, UnixTimestamp},
    },
    solana_transaction_status::{
        Rewards,
    },
    serde_derive::{Serialize,Deserialize},
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UiConfirmedBlock {
    pub previous_blockhash: String,
    pub blockhash: String,
    pub parent_slot: Slot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<EncodedTransactionWithStatusMeta>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rewards: Option<Rewards>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_reward_partitions: Option<u64>,
    pub block_time: Option<UnixTimestamp>,
    pub block_height: Option<u64>,
}

impl From<EncodedConfirmedBlock> for UiConfirmedBlock {
    fn from(block: EncodedConfirmedBlock) -> Self {
        Self {
            previous_blockhash: block.previous_blockhash,
            blockhash: block.blockhash,
            parent_slot: block.parent_slot,
            transactions: Some(block.transactions),
            signatures: None, // Set to None since it's not available in EncodedConfirmedBlock
            rewards: Some(block.rewards),
            num_reward_partitions: block.num_partitions,
            block_time: block.block_time,
            block_height: block.block_height,
        }
    }
}