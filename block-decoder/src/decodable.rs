
use {
    crate::{
        errors::{
            decode_error::DecodeError,
        }
    },
    solana_sdk::{
        transaction::{
            TransactionVersion,
        },
    },
    solana_transaction_status::{
        UiTransactionEncoding,
    },
};

pub trait Decodable {
    type Encoded;
    type Decoded;
    fn decode(encoded: &Self::Encoded) -> Result<Self::Decoded, DecodeError>;
}

pub trait DecodableWithMeta {
    type Encoded;
    type Decoded;
    fn decode_with_meta(
        encoded: Self::Encoded,
        encoding: UiTransactionEncoding,
        version: Option<TransactionVersion>
    ) -> Result<Self::Decoded, DecodeError>;
    fn json_decode(encoded: Self::Encoded, version: Option<TransactionVersion>) -> Result<Self::Decoded, DecodeError>;
}
