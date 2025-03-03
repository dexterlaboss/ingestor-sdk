use {
    solana_pubkey::{
        Pubkey,
    },
    solana_message::{
        VersionedMessage,
    },
    solana_transaction::{
        Transaction,
    },
    solana_transaction_status::{
        VersionedTransactionWithStatusMeta,
        TransactionWithStatusMeta,
    },
    std::{
        str::FromStr,
    },
};

pub(crate) fn is_voting_tx(transaction_with_status_meta: &TransactionWithStatusMeta) -> bool {
    let account_address = Pubkey::from_str("Vote111111111111111111111111111111111111111").unwrap();

    match transaction_with_status_meta {
        TransactionWithStatusMeta::MissingMetadata(transaction) => {
            has_account_in_transaction(transaction, &account_address)
        }
        TransactionWithStatusMeta::Complete(versioned_transaction_with_meta) => {
            has_account(versioned_transaction_with_meta, &account_address)
        }
    }
}

pub(crate) fn has_account(
    versioned_transaction_with_meta: &VersionedTransactionWithStatusMeta,
    address: &Pubkey
) -> bool {
    match &versioned_transaction_with_meta.transaction.message {
        VersionedMessage::Legacy(message) => message.account_keys.contains(address),
        VersionedMessage::V0(message) => message.account_keys.contains(address),
    }
}

pub(crate) fn has_account_in_transaction(transaction: &Transaction, address: &Pubkey) -> bool {
    transaction.message.account_keys.contains(address)
}

pub(crate) fn is_error_tx(transaction_with_status_meta: &TransactionWithStatusMeta) -> bool {
    match transaction_with_status_meta {
        TransactionWithStatusMeta::MissingMetadata(_) => {
            false
        }
        TransactionWithStatusMeta::Complete(versioned_transaction_with_meta) => {
            versioned_transaction_with_meta.meta.status.is_err()
        }
    }
}

pub(crate) fn determine_transaction_type(transaction_with_status_meta: &TransactionWithStatusMeta) -> &'static str {
    if is_voting_tx(transaction_with_status_meta) {
        "voting"
    } else if is_error_tx(transaction_with_status_meta) {
        "error"
    } else {
        "regular"
    }
}

pub(crate) fn calculate_epoch(slot: u64) -> u64 {
    slot / 432_000
}