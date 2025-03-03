use {
    crate::{
        stored_confirmed_block::StoredConfirmedBlockTransaction,
    },
    serde::{Deserialize, Serialize},
    // solana_sdk::{
    //     clock::{Slot, UnixTimestamp},
    // },
    solana_clock::{
        Slot,
        UnixTimestamp,
    },
    solana_transaction_status::{
        ConfirmedTransactionWithStatusMeta,
    },
};

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