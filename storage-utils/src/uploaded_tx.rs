
use {
    crate::{
        tx_info::TransactionInfo,
    },
    solana_clock::{
        Slot,
    },
    solana_transaction_error::{
        TransactionError,
    },
};

#[derive(PartialEq, Eq, Debug)]
pub struct UploadedTransaction {
    pub slot: Slot, // The slot that contains the block with this transaction in it
    pub index: u32, // Where the transaction is located in the block
    pub err: Option<TransactionError>, // None if the transaction executed successfully
}

impl From<TransactionInfo> for UploadedTransaction {
    fn from(transaction_info: TransactionInfo) -> Self {
        Self {
            slot: transaction_info.slot,
            index: transaction_info.index,
            err: transaction_info.err,
        }
    }
}