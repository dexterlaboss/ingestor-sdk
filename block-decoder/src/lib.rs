use {
    solana_storage_utils::compression::compress_best,
    solana_transaction_status::{
        BlockEncodingOptions,
        VersionedConfirmedBlock as SolanaVersionedConfirmedBlock,
    },
    solana_transaction_status_client_types::{
        UiTransactionEncoding,
    },
    block::{
        confirmed_block::ConfirmedBlock,
        encoded_block::EncodedConfirmedBlock,
        ui_block::UiConfirmedBlock,
        versioned_block::VersionedConfirmedBlock,
    },
    std::sync::atomic::{AtomicBool, Ordering},
};

pub mod errors {
    pub mod conversion_error;
    pub mod decode_error;
}

pub mod block {
    pub mod confirmed_block;
    pub mod encoded_block;
    pub mod versioned_block;
    pub mod ui_block;
}

pub mod transaction {
    pub mod transaction;
    pub mod tx_status_meta;
    pub mod tx_return_data;
    pub mod tx_token_balance;
    pub mod versioned_transaction;

    pub use transaction::Transaction;
}

pub mod address {
    pub mod address_table_lookup;
    pub mod loaded_addresses;
}

pub mod instruction {
    pub mod compiled_instruction;
    pub mod inner_instruction;
    pub use compiled_instruction::CompiledInstruction;
    pub use inner_instruction::InnerInstruction;
}

pub mod message {
    pub mod message;
    pub mod message_v0;
    pub mod versioned_message;
}

pub mod decodable;

// Global runtime toggle that controls whether empty metadata should be injected
// for transactions missing metadata when converting blocks.
lazy_static::lazy_static! {
    static ref ADD_EMPTY_TX_METADATA_IF_MISSING: AtomicBool = AtomicBool::new(
        std::env::var("ADD_EMPTY_TX_METADATA_IF_MISSING")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    );
}

/// Enable or disable injecting empty metadata for transactions that are missing it.
pub fn set_add_empty_tx_metadata_if_missing(value: bool) {
    ADD_EMPTY_TX_METADATA_IF_MISSING.store(value, Ordering::Relaxed);
}

/// Read the current setting for injecting empty metadata for missing transaction metadata.
pub fn add_empty_tx_metadata_if_missing() -> bool {
    ADD_EMPTY_TX_METADATA_IF_MISSING.load(Ordering::Relaxed)
}

pub async fn encode_block<T>(
    data: T,
) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    where
        T: prost::Message,
{
    let mut buf = Vec::with_capacity(data.encoded_len());
    data.encode(&mut buf).unwrap();
    let data = compress_best(&buf)?;

    Ok(data)
}

pub fn convert_block(
    encoded_block: EncodedConfirmedBlock,
    encoding: UiTransactionEncoding,
    options: BlockEncodingOptions,
) -> Result<SolanaVersionedConfirmedBlock, Box<dyn std::error::Error>> {
    // Step 1: Convert EncodedConfirmedBlock to UiConfirmedBlock
    let ui_block: UiConfirmedBlock = encoded_block.into();

    // Step 2: Decode UiConfirmedBlock to ConfirmedBlock
    let confirmed_block = ConfirmedBlock::decode_with_options(ui_block, encoding, options)?;

    // Step 3: Try to convert ConfirmedBlock to VersionedConfirmedBlock
    let versioned_block = VersionedConfirmedBlock::try_from(confirmed_block)?;

    // Ok(convert_versioned_confirmed_block(&versioned_block))
    Ok(versioned_block.into())
}

pub async fn encode_transaction<T>(
    data: T,
) -> Result<Vec<u8>, Box<dyn std::error::Error>>
    where
        T: prost::Message,
{
    let mut buf = Vec::with_capacity(data.encoded_len());
    data.encode(&mut buf).unwrap();
    let data = compress_best(&buf)?;

    Ok(data)
}

// pub fn convert_transaction(
//     encoded_tx: EncodedTransactionWithStatusMeta,
//     encoding: UiTransactionEncoding,
//     // options: BlockEncodingOptions,
// ) -> Result<TransactionWithStatusMeta, Box<dyn std::error::Error>> {
//
//     let confirmed_tx = TransactionWithStatusMeta::decode(encoded_tx, encoding)?;
//
//     // Try to convert ConfirmedBlock to VersionedConfirmedBlock
//     // let versioned_block = VersionedConfirmedBlock::try_from(confirmed_tx)?;
//
//     Ok(confirmed_tx)
// }
