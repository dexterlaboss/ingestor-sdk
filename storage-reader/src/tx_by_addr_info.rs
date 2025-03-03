
use {
    serde::{Deserialize, Serialize},
    solana_signature::{
        Signature,
    },
    solana_transaction_error::{
        TransactionError,
    },
    solana_transaction_status::{
        TransactionByAddrInfo,
    },
};

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