use {
    crate::{
        errors::{
            decode_error::DecodeError,
        },
        block::{
            encoded_block::{
                EncodedTransaction,
                EncodedTransactionWithStatusMeta,
            },
        },
        message::{
            message::Message,
            message_v0::Message as MessageV0,
            versioned_message::VersionedMessage,
        },
        transaction::{
            tx_status_meta::TransactionStatusMeta,
        },
        decodable::{
            Decodable,
            DecodableWithMeta,
        },
    },
    serde::{
        Deserialize, Serialize,
    },
    solana_short_vec as short_vec,
    solana_signature::Signature,
    solana_transaction::{
        versioned::TransactionVersion
    },
    solana_transaction_status_client_types::{
        UiMessage,
        UiTransactionEncoding,
    },
    base64::{Engine, prelude::BASE64_STANDARD},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VersionedTransactionWithStatusMeta {
    pub transaction: VersionedTransaction,
    pub meta: TransactionStatusMeta,
}

impl VersionedTransactionWithStatusMeta {
    pub fn decode(
        encoded: EncodedTransactionWithStatusMeta,
        encoding: UiTransactionEncoding,
    ) -> Result<Self, DecodeError> {
        // Decoding the transaction
        let transaction = match VersionedTransaction::decode_with_meta(encoded.transaction, encoding, encoded.version /*, meta*/) {
            Ok(decoded) => decoded,
            Err(e) => return Err(e),
        };

        // Decoding the meta
        let meta = match encoded.meta {
            Some(ui_meta) => match TransactionStatusMeta::try_from(ui_meta) {
                Ok(meta) => meta,
                Err(_) => return Err(DecodeError::InvalidData),
            },
            None => return Err(DecodeError::InvalidData),
        };

        Ok(Self {
            transaction,
            meta,
        })
    }
}

#[derive(Debug, PartialEq, Default, Eq, Clone, Serialize, Deserialize)]
pub struct VersionedTransaction {
    /// List of signatures
    #[serde(with = "short_vec")]
    pub signatures: Vec<Signature>,
    /// Message to sign.
    pub message: VersionedMessage,
}


impl DecodableWithMeta for VersionedTransaction {
    type Encoded = EncodedTransaction;
    type Decoded = VersionedTransaction;

    fn decode_with_meta(
        encoded: Self::Encoded,
        decoding: UiTransactionEncoding,
        version: Option<TransactionVersion>
    ) -> Result<Self::Decoded, DecodeError> {
        match decoding {
            UiTransactionEncoding::Binary | UiTransactionEncoding::Base58 => {
                if let EncodedTransaction::LegacyBinary(encoded_string) = encoded {
                    let decoded_bytes = bs58::decode(encoded_string).into_vec().unwrap();
                    let decoded: Self::Decoded =
                        bincode::deserialize(&decoded_bytes).map_err(|_| DecodeError::DeserializeFailed)?;
                    Ok(decoded)
                } else {
                    Err(DecodeError::UnsupportedEncoding)
                }
            }
            UiTransactionEncoding::Base64 => {
                if let EncodedTransaction::Binary(encoded_string, _) = encoded {
                    let decoded_bytes = BASE64_STANDARD.decode(encoded_string).unwrap();
                    let decoded: Self::Decoded =
                        bincode::deserialize(&decoded_bytes).map_err(|_| DecodeError::DeserializeFailed)?;
                    Ok(decoded)
                } else {
                    Err(DecodeError::UnsupportedEncoding)
                }
            }
            UiTransactionEncoding::Json => Self::json_decode(encoded, version),
            UiTransactionEncoding::JsonParsed => Err(DecodeError::UnsupportedEncoding),
        }
    }

    fn json_decode(encoded: Self::Encoded, version: Option<TransactionVersion>) -> Result<Self::Decoded, DecodeError> {
        if let EncodedTransaction::Json(ui_transaction) = encoded {
            let signatures = ui_transaction
                .signatures
                .iter()
                .map(|s| s.parse::<Signature>())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|err| DecodeError::ParseSignatureFailed(err))?;

            let message = match ui_transaction.message {
                UiMessage::Raw(_) => {
                    match version {
                        Some(TransactionVersion::Number(0)) => {
                            // Handle Version 0 message decoding for raw messages
                            let v0_message = MessageV0::json_decode(ui_transaction.message, version)?;
                            VersionedMessage::V0(v0_message)
                        }
                        Some(TransactionVersion::Legacy(_)) | None => {
                            // Default to legacy message decoding for raw messages
                            let legacy_message = Message::decode(&ui_transaction.message)?;
                            VersionedMessage::Legacy(legacy_message)
                        }
                        // Add additional cases here for other versions as needed
                        _ => {
                            // Handle other versions or return an error if not supported
                            return Err(DecodeError::UnsupportedVersion);
                        }
                    }
                }
                UiMessage::Parsed(_) => {
                    return Err(DecodeError::UnsupportedEncoding);
                }
            };

            Ok(Self {
                signatures,
                message,
            })
        } else {
            Err(DecodeError::UnsupportedEncoding)
        }
    }
}

impl From<VersionedTransaction> for solana_transaction::versioned::VersionedTransaction {
    fn from(tx: VersionedTransaction) -> Self {
        Self {
            signatures: tx.signatures,
            message: tx.message.into(),
        }
    }
}

impl From<VersionedTransactionWithStatusMeta> for solana_transaction_status::VersionedTransactionWithStatusMeta {
    fn from(tx: VersionedTransactionWithStatusMeta) -> Self {
        Self {
            transaction: tx.transaction.into(),
            meta: tx.meta.into(),
        }
    }
}
