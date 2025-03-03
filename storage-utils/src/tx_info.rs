
use {
    serde::{Deserialize, Serialize},
    solana_clock::{
        Slot,
    },
    solana_transaction_error::{
        TransactionError,
    },
    solana_transaction_status::{
        TransactionConfirmationStatus,
        TransactionStatus,
    },
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct TransactionInfo {
    pub slot: Slot, // The slot that contains the block with this transaction in it
    pub index: u32, // Where the transaction is located in the block
    pub err: Option<TransactionError>, // None if the transaction executed successfully
    // pub memo: Option<String>, // Transaction memo
}

impl From<TransactionInfo> for TransactionStatus {
    fn from(transaction_info: TransactionInfo) -> Self {
        let TransactionInfo { slot, err, .. } = transaction_info;
        let status = match &err {
            None => Ok(()),
            Some(err) => Err(err.clone()),
        };
        Self {
            slot,
            confirmations: None,
            status,
            err,
            confirmation_status: Some(TransactionConfirmationStatus::Finalized),
        }
    }
}