use {
    // solana_sdk::{
    //     pubkey::{ParsePubkeyError},
    //     signature::ParseSignatureError,
    //     hash::{ParseHashError},
    // },
    solana_pubkey::ParsePubkeyError,
    solana_signature::ParseSignatureError,
    solana_hash::ParseHashError,
    std::{
        error::Error,
        fmt,
    },
};

#[derive(Debug, PartialEq)]
pub enum DecodeError {
    InvalidEncoding,
    InvalidAccountKey,
    InvalidBlockhash,
    DecodeFailed,
    DeserializeFailed,
    ParseSignatureFailed(ParseSignatureError),
    ParseHashFailed(ParseHashError),
    ParsePubkeyFailed(ParsePubkeyError),
    NotImplemented,
    InvalidData,
    UnsupportedEncoding,
    UnsupportedVersion,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::InvalidEncoding => write!(f, "Invalid encoding"),
            DecodeError::DecodeFailed => write!(f, "Decoding failed"),
            DecodeError::DeserializeFailed => write!(f, "Deserialization failed"),
            DecodeError::InvalidAccountKey => write!(f, "Invalid account key"),
            DecodeError::InvalidBlockhash => write!(f, "Invalid blockhash"),
            DecodeError::ParseSignatureFailed(err) => write!(f, "Failed to parse signature: {}", err),
            DecodeError::ParseHashFailed(err) => write!(f, "Failed to parse hash: {}", err),
            DecodeError::ParsePubkeyFailed(err) => write!(f, "Failed to parse pubkey: {}", err),
            DecodeError::NotImplemented => write!(f, "Not implemented"),
            DecodeError::InvalidData => write!(f, "Invalid data"),
            DecodeError::UnsupportedEncoding => write!(f, "Encoding is not supported"),
            DecodeError::UnsupportedVersion => write!(f, "Transaction version is not supported"),
        }
    }
}

impl Error for DecodeError {}

impl From<ParsePubkeyError> for DecodeError {
    fn from(err: ParsePubkeyError) -> Self {
        DecodeError::ParsePubkeyFailed(err)
    }
}