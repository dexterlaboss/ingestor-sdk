use {
    solana_storage_utils::compression::compress_best,
    solana_transaction_status::{
        BlockEncodingOptions,
        // UiTransactionEncoding,
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
