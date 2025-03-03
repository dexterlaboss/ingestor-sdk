#![allow(clippy::integer_arithmetic)]

use {
    // solana_sdk::{
    //     pubkey::Pubkey,
    //     message::{
    //         VersionedMessage,
    //         v0::LoadedAddresses,
    //     },
    //     instruction::CompiledInstruction,
    // },
    solana_pubkey::{
        Pubkey,
    },
    solana_message::{
        VersionedMessage,
        v0::LoadedAddresses,
        compiled_instruction::CompiledInstruction,
    },
    solana_transaction_status::{
        VersionedTransactionWithStatusMeta,
        TransactionWithStatusMeta,
    },
    std::{
        str::FromStr,
    },
};

pub(crate) fn get_account_keys(transaction_with_meta: &VersionedTransactionWithStatusMeta) -> Vec<Pubkey> {
    match &transaction_with_meta.transaction.message {
        VersionedMessage::V0(_) => {
            let static_keys = transaction_with_meta.transaction.message.static_account_keys();
            let LoadedAddresses { writable, readonly } = &transaction_with_meta.meta.loaded_addresses;

            static_keys.iter()
                .chain(writable.iter())
                .chain(readonly.iter())
                .cloned()
                .collect()
        },
        VersionedMessage::Legacy(_) => {
            Vec::from(transaction_with_meta.transaction.message.static_account_keys())
        }
    }
}

pub(crate) fn is_error_tx(transaction_with_meta: &VersionedTransactionWithStatusMeta) -> bool {
    transaction_with_meta.meta.status.is_err()
}

pub(crate) fn is_voting_tx(transaction_with_meta: &VersionedTransactionWithStatusMeta) -> bool {
    let account_address = Pubkey::from_str("Vote111111111111111111111111111111111111111").unwrap();

    has_account(transaction_with_meta, &account_address)
}

pub(crate) fn has_account(transaction_with_meta: &VersionedTransactionWithStatusMeta, address: &Pubkey) -> bool {
    transaction_with_meta
        .transaction
        .message
        .static_account_keys()
        .contains(&address)
}

pub(crate) fn is_program_account(
    address: &Pubkey,
    transaction_with_meta: &VersionedTransactionWithStatusMeta,
    combined_keys: &[Pubkey]
) -> bool {
    // Helper to check if the address is used as a program account in a given instruction
    let check_program_id = |instruction: &CompiledInstruction, account_keys: &[Pubkey]| -> bool {
        let program_id = &account_keys[instruction.program_id_index as usize];
        program_id == address
    };

    // Check in outer instructions
    let used_in_outer = transaction_with_meta.transaction.message.instructions().iter().any(|instruction| {
        check_program_id(instruction, combined_keys)
    });

    // Check in inner instructions
    let used_in_inner = transaction_with_meta.meta.inner_instructions.as_ref()
        .map_or(false, |inner_instructions| {
            inner_instructions.iter().flat_map(|inner| &inner.instructions)
                .any(|inner_instruction| check_program_id(&inner_instruction.instruction, combined_keys))
        });

    used_in_outer || used_in_inner
}

//----------------------

pub(crate) fn convert_to_transaction_with_status_meta(item: VersionedTransactionWithStatusMeta) -> TransactionWithStatusMeta {
    TransactionWithStatusMeta::Complete(item)
}